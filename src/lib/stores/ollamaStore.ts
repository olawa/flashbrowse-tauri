import { writable, get } from 'svelte/store';

export interface OllamaModelInfo {
  name: string;
  size: number;
  family?: string;
  parameter_size?: string;
  quantization_level?: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
}

export const ollamaEndpoint = writable<string>('http://127.0.0.1:11434');
export const isOllamaOnline = writable<boolean>(false);
export const installedModels = writable<OllamaModelInfo[]>([]);
export const selectedModel = writable<string>('');
export const isAiGenerating = writable<boolean>(false);
export const aiChatMessages = writable<ChatMessage[]>([]);

let currentAbortController: AbortController | null = null;

export async function checkOllamaConnection(): Promise<boolean> {
  const endpoint = get(ollamaEndpoint);
  try {
    const res = await fetch(`${endpoint}/api/tags`, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
    });
    if (!res.ok) {
      isOllamaOnline.set(false);
      return false;
    }
    const data = await res.json();
    const models: OllamaModelInfo[] = (data.models || []).map((m: any) => ({
      name: m.name,
      size: m.size || 0,
      family: m.details?.family || '',
      parameter_size: m.details?.parameter_size || '',
      quantization_level: m.details?.quantization_level || '',
    }));

    installedModels.set(models);
    isOllamaOnline.set(true);

    // Pick a smart default model if none is selected
    const current = get(selectedModel);
    if (!current || !models.some((m) => m.name === current)) {
      const preferred =
        models.find((m) => m.name.includes('coder')) ||
        models.find((m) => m.name.includes('qwen')) ||
        models.find((m) => m.name.includes('gemma')) ||
        models[0];
      if (preferred) {
        selectedModel.set(preferred.name);
      }
    }
    return true;
  } catch (err) {
    isOllamaOnline.set(false);
    return false;
  }
}

export async function askOllamaStream(
  prompt: string,
  systemContext = '',
  fileContext = ''
): Promise<void> {
  const endpoint = get(ollamaEndpoint);
  const model = get(selectedModel);

  if (!model) {
    throw new Error('Ingen modell vald.');
  }

  // Cancel any existing running request
  if (currentAbortController) {
    currentAbortController.abort();
  }
  currentAbortController = new AbortController();

  const userMsgId = 'msg-' + Date.now();
  const assistantMsgId = 'msg-' + (Date.now() + 1);

  // Add User Message
  aiChatMessages.update((msgs) => [
    ...msgs,
    {
      id: userMsgId,
      role: 'user',
      content: prompt,
      timestamp: Date.now(),
    },
    {
      id: assistantMsgId,
      role: 'assistant',
      content: '',
      timestamp: Date.now() + 1,
    },
  ]);

  isAiGenerating.set(true);

  try {
    let fullPrompt = prompt;
    if (fileContext) {
      fullPrompt = `Filinnehåll / Kontext:\n\`\`\`\n${fileContext.slice(0, 24000)}\n\`\`\`\n\nFråga / Instruktion:\n${prompt}`;
    }

    const res = await fetch(`${endpoint}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model,
        prompt: fullPrompt,
        system:
          systemContext ||
          'Du är en snabb, hjälpsam AI-assistent integrerad i filutforskaren Flashbrowse. Svara koncist och tydligt på svenska med snygg GitHub-formaterad Markdown.',
        stream: true,
      }),
      signal: currentAbortController.signal,
    });

    if (!res.ok || !res.body) {
      throw new Error(`Ollama svarade med felkod: ${res.status}`);
    }

    const reader = res.body.getReader();
    const decoder = new TextDecoder('utf-8');
    let accumulatedText = '';
    let buffer = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const json = JSON.parse(trimmed);
          if (json.response) {
            accumulatedText += json.response;
            aiChatMessages.update((msgs) => {
              const last = msgs[msgs.length - 1];
              if (last && last.id === assistantMsgId) {
                last.content = accumulatedText;
              }
              return [...msgs];
            });
          }
        } catch (e) {
          // ignore partial json
        }
      }
    }
  } catch (err: any) {
    if (err.name !== 'AbortError') {
      console.error('Ollama stream error:', err);
      aiChatMessages.update((msgs) => {
        const last = msgs[msgs.length - 1];
        if (last && last.id === assistantMsgId) {
          last.content += `\n\n*(⚠️ Fel vid anslutning till Ollama: ${err.message})*`;
        }
        return [...msgs];
      });
    }
  } finally {
    isAiGenerating.set(false);
    currentAbortController = null;
  }
}

export function stopAiGeneration() {
  if (currentAbortController) {
    currentAbortController.abort();
    currentAbortController = null;
  }
  isAiGenerating.set(false);
}

export function clearAiChat() {
  stopAiGeneration();
  aiChatMessages.set([]);
}
