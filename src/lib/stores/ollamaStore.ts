import { writable, get } from 'svelte/store';

export interface OllamaModelInfo {
  name: string;
  size: number;
  family?: string;
  parameter_size?: string;
  quantization_level?: string;
}

export interface RunningModelInfo {
  name: string;
  size: number;
  formatted_size: string;
  processor: string;
  expires_at?: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
  }
  if (bytes >= 1024 * 1024) {
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
  }
  return bytes + ' B';
}

export const ollamaEndpoint = writable<string>('http://127.0.0.1:11434');
export const isOllamaOnline = writable<boolean>(false);
export const installedModels = writable<OllamaModelInfo[]>([]);
export const runningModels = writable<RunningModelInfo[]>([]);
export const selectedModel = writable<string>('');
export const isAiGenerating = writable<boolean>(false);
export const aiChatMessages = writable<ChatMessage[]>([]);

let currentAbortController: AbortController | null = null;

export async function checkRunningModels(): Promise<RunningModelInfo[]> {
  const endpoint = get(ollamaEndpoint);
  try {
    const res = await fetch(`${endpoint}/api/ps`, {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
    });
    if (!res.ok) {
      runningModels.set([]);
      return [];
    }
    const data = await res.json();
    const list: RunningModelInfo[] = (data.models || []).map((m: any) => ({
      name: m.name || m.model || '',
      size: m.size || 0,
      formatted_size: formatBytes(m.size || 0),
      processor: m.processor || 'GPU',
      expires_at: m.expires_at,
    }));
    runningModels.set(list);
    return list;
  } catch {
    runningModels.set([]);
    return [];
  }
}

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

    await checkRunningModels();

    // Pick a smart lightweight default model if none is selected
    const saved = localStorage.getItem('flashbrowse_selected_model');
    const current = get(selectedModel);
    if (saved && models.some((m) => m.name === saved)) {
      selectedModel.set(saved);
    } else if (!current || !models.some((m) => m.name === current)) {
      // Sort models by size to pick the fastest small model (e.g. 3B - 8B)
      const sortedBySize = [...models].sort((a, b) => a.size - b.size);
      const preferred =
        sortedBySize.find((m) => m.name.includes('llama3.2:3b') || m.name.includes('llama3.2:1b')) ||
        sortedBySize.find((m) => m.name.includes('qwen2.5-coder:7b') || m.name.includes('qwen2.5:3b') || m.name.includes('qwen2.5:7b')) ||
        sortedBySize.find((m) => m.name === 'gemma4:latest' || m.name.includes('gemma4:latest') || m.name.includes(':8b')) ||
        sortedBySize.find((m) => m.name.includes('phi')) ||
        sortedBySize[0];
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

export async function unloadOllamaModel(modelName: string): Promise<void> {
  const endpoint = get(ollamaEndpoint);
  try {
    await fetch(`${endpoint}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: modelName,
        keep_alive: 0,
      }),
    });
    await checkRunningModels();
  } catch (err) {
    console.warn('Failed to unload model:', err);
  }
}

export async function unloadAllOllamaModels(): Promise<void> {
  const running = get(runningModels);
  for (const m of running) {
    await unloadOllamaModel(m.name);
  }
  // Extra check
  await checkRunningModels();
}

export async function selectModelWithAutoEvict(newModel: string): Promise<void> {
  const oldModel = get(selectedModel);
  if (oldModel && oldModel !== newModel) {
    // Unload the old model to free RAM
    await unloadOllamaModel(oldModel);
  }
  selectedModel.set(newModel);
  try {
    localStorage.setItem('flashbrowse_selected_model', newModel);
  } catch {}
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
        keep_alive: '2m', // Auto-evict from VRAM after 2 minutes of idle
        stream: true,
      }),
      signal: currentAbortController.signal,
    });

    if (!res.ok || !res.body) {
      throw new Error(`Ollama svarade med felkod: ${res.status}`);
    }

    // Refresh running models in background
    setTimeout(() => checkRunningModels(), 1000);

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
    await checkRunningModels();
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
