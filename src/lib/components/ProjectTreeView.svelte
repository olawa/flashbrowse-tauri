<script context="module" lang="ts">
  import type { FileItem } from '../types';

  export interface TreeNode {
    name: string;
    path: string;
    isDir: boolean;
    item: FileItem;
    children: TreeNode[];
    sizeFormatted?: string;
    modifiedFormatted?: string;
  }

  export interface DetectedProject {
    id: string;
    name: string;
    path: string;
    relativePath: string;
    type: 'rust' | 'python' | 'pipeline' | 'web' | 'script';
    typeLabel: string;
    badge: string;
    colorClass: string;
    icon: any;
    manifest: string;
    fileCount: number;
    item: FileItem;
    tree: TreeNode[];
  }
</script>

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { DirectoryIndexGroup } from '../types';
  import { favoriteEditor, openInFavoriteEditor } from '../stores/editorStore';
  import { activeHoveredItem } from '../stores/navigation';
  import { revealInOs } from '../invoke';
  import {
    Folder,
    FolderOpen,
    FileCode,
    FileText,
    ChevronRight,
    ChevronDown,
    ExternalLink,
    Code,
    GitBranch,
    Cpu,
    Terminal,
    Globe,
  } from 'lucide-svelte';

  // Root Props
  export let groups: DirectoryIndexGroup[] = [];
  export let rootPath: string = '';
  export let searchQuery: string = '';
  export let onSelectPreview: (item: FileItem) => void = () => {};
  export let onNavigateFolder: (dirPath: string) => void = () => {};

  // Recursive Props for sub-trees
  export let treeNodes: TreeNode[] | undefined = undefined;
  export let expandedPaths: Set<string> = new Set<string>();
  export let toggleFolder: (path: string, e?: MouseEvent) => void = () => {};
  export let handleFolderMouseEnter: (path: string, folderItem?: FileItem) => void = () => {};
  export let handleFolderMouseLeave: () => void = () => {};
  export let handleFileHover: (item?: FileItem) => void = () => {};
  export let handleFileMouseLeave: () => void = () => {};

  let localExpandedPaths = new Set<string>();
  let expandedCategoryTypes = new Set<string>(['rust', 'pipeline', 'python', 'web', 'script']);
  let hoverTimer: any = null;
  let hoverPreviewTimer: any = null;
  let hoveredPath: string | null = null;
  let collapsedByClick = new Set<string>();

  onDestroy(() => {
    clearTimeout(hoverTimer);
    clearTimeout(hoverPreviewTimer);
  });

  // Only run analysis if we are at root level
  $: detectedProjects = !treeNodes ? analyzeProjects(groups, rootPath, searchQuery) : [];

  // Auto-expand all detected project roots initially so the first level is immediately visible
  let hasAutoExpandedRoots = false;
  $: if (detectedProjects.length > 0 && !hasAutoExpandedRoots) {
    hasAutoExpandedRoots = true;
    const initial = new Set(localExpandedPaths);
    for (const p of detectedProjects) {
      initial.add(p.path);
    }
    localExpandedPaths = initial;
  }

  // Auto-expand paths when user enters a search query
  $: if (searchQuery.trim() && detectedProjects.length > 0) {
    const query = searchQuery.trim().toLowerCase();
    const expanded = new Set(localExpandedPaths);
    for (const proj of detectedProjects) {
      expandMatchingNodes(proj.tree, query, expanded);
    }
    localExpandedPaths = expanded;
  }

  function expandMatchingNodes(nodes: TreeNode[], query: string, expanded: Set<string>): boolean {
    let hasMatch = false;
    for (const node of nodes) {
      const selfMatch = node.name.toLowerCase().includes(query);
      let childMatch = false;
      if (node.children.length > 0) {
        childMatch = expandMatchingNodes(node.children, query, expanded);
      }
      if (selfMatch || childMatch) {
        hasMatch = true;
        if (node.isDir) {
          expanded.add(node.path);
        }
      }
    }
    return hasMatch;
  }

  $: categorizedProjects = {
    rust: detectedProjects.filter((p) => p.type === 'rust'),
    pipeline: detectedProjects.filter((p) => p.type === 'pipeline'),
    python: detectedProjects.filter((p) => p.type === 'python'),
    web: detectedProjects.filter((p) => p.type === 'web'),
    script: detectedProjects.filter((p) => p.type === 'script'),
  };

  function analyzeProjects(allGroups: DirectoryIndexGroup[], baseRoot: string, queryStr: string): DetectedProject[] {
    const rawFiles: FileItem[] = [];
    for (const g of allGroups) {
      rawFiles.push(...g.items);
    }
    if (rawFiles.length === 0) return [];

    const q = queryStr.trim().toLowerCase();
    const allFiles = q
      ? rawFiles.filter((f) => f.name.toLowerCase().includes(q) || f.path.toLowerCase().includes(q))
      : rawFiles;

    if (allFiles.length === 0) return [];

    const projectRoots = new Map<string, { type: 'rust' | 'python' | 'pipeline' | 'web'; manifest: string }>();

    for (const file of allFiles) {
      const lower = file.name.toLowerCase();
      const parentDir = file.path.substring(0, file.path.lastIndexOf('/')) || file.path;

      if (lower === 'cargo.toml') {
        if (parentDir.endsWith('/src-tauri')) {
          const grandParent = parentDir.substring(0, parentDir.lastIndexOf('/'));
          if (!projectRoots.has(grandParent)) {
            projectRoots.set(grandParent, { type: 'rust', manifest: 'Cargo.toml' });
          }
        } else {
          projectRoots.set(parentDir, { type: 'rust', manifest: 'Cargo.toml' });
        }
      } else if (['snakefile', 'snakefile.smk', 'nextflow.config', 'main.nf'].includes(lower) || lower.endsWith('.smk')) {
        projectRoots.set(parentDir, { type: 'pipeline', manifest: file.name });
      } else if (['pyproject.toml', 'requirements.txt', 'environment.yml', 'setup.py', 'pipfile'].includes(lower) || lower.endsWith('.ipynb')) {
        if (!projectRoots.has(parentDir)) {
          projectRoots.set(parentDir, { type: 'python', manifest: file.name });
        }
      } else if (lower === 'package.json' && !projectRoots.has(parentDir)) {
        projectRoots.set(parentDir, { type: 'web', manifest: 'package.json' });
      }
    }

    const sortedRoots = Array.from(projectRoots.entries()).sort(
      (a, b) => a[0].length - b[0].length
    );

    const consolidatedRoots = new Map<string, { type: 'rust' | 'python' | 'pipeline' | 'web'; manifest: string }>();
    for (const [rPath, meta] of sortedRoots) {
      let hasParentRoot = false;
      for (const existingRoot of consolidatedRoots.keys()) {
        if (rPath.startsWith(existingRoot + '/')) {
          hasParentRoot = true;
          break;
        }
      }
      if (!hasParentRoot) {
        consolidatedRoots.set(rPath, meta);
      }
    }

    const projectFilesMap = new Map<string, FileItem[]>();
    const looseFiles: FileItem[] = [];

    for (const file of allFiles) {
      let assigned = false;
      for (const rPath of consolidatedRoots.keys()) {
        if (file.path === rPath || file.path.startsWith(rPath + '/')) {
          const list = projectFilesMap.get(rPath) || [];
          list.push(file);
          projectFilesMap.set(rPath, list);
          assigned = true;
          break;
        }
      }
      if (!assigned) {
        looseFiles.push(file);
      }
    }

    const projects: DetectedProject[] = [];

    for (const [rPath, meta] of consolidatedRoots.entries()) {
      const pFiles = projectFilesMap.get(rPath) || [];
      const pName = rPath.split('/').pop() || rPath;
      const relPath = rPath.startsWith(baseRoot) ? '.' + rPath.slice(baseRoot.length) : rPath;

      let typeLabel = 'Rust';
      let badge = 'RUST';
      let colorClass = 'text-orange-400';
      let icon = Cpu;

      if (meta.type === 'pipeline') {
        typeLabel = 'Pipeline';
        badge = 'PIPE';
        colorClass = 'text-emerald-400';
        icon = GitBranch;
      } else if (meta.type === 'python') {
        typeLabel = 'Python';
        badge = 'PY';
        colorClass = 'text-yellow-400';
        icon = FileCode;
      } else if (meta.type === 'web') {
        typeLabel = 'TypeScript & Web';
        badge = 'WEB';
        colorClass = 'text-sky-400';
        icon = Globe;
      }

      const tree = buildTreeFromFiles(rPath, pFiles);

      const projectItem: FileItem = {
        name: pName,
        path: rPath,
        is_dir: true,
        is_symlink: false,
        is_hidden: false,
        size_bytes: 0,
        formatted_size: '--',
        modified_timestamp: 0,
        formatted_modified: '--',
        permissions: 'rwxr-xr-x',
        extension: '',
      };

      projects.push({
        id: rPath,
        name: pName,
        path: rPath,
        relativePath: relPath,
        type: meta.type,
        typeLabel,
        badge,
        colorClass,
        icon,
        manifest: meta.manifest,
        fileCount: pFiles.length,
        item: projectItem,
        tree,
      });
    }

    projects.sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true }));

    if (looseFiles.length > 0) {
      const looseTree = buildTreeFromFiles(baseRoot, looseFiles);
      const looseItem: FileItem = {
        name: 'Fristående skript & verktyg',
        path: baseRoot,
        is_dir: true,
        is_symlink: false,
        is_hidden: false,
        size_bytes: 0,
        formatted_size: '--',
        modified_timestamp: 0,
        formatted_modified: '--',
        permissions: 'rwxr-xr-x',
        extension: '',
      };
      projects.push({
        id: 'loose_scripts',
        name: 'Fristående skript & verktyg',
        path: baseRoot,
        relativePath: './scripts',
        type: 'script',
        typeLabel: 'Skript',
        badge: 'SCRIPT',
        colorClass: 'text-cyan-400',
        icon: Terminal,
        manifest: 'Skript',
        fileCount: looseFiles.length,
        item: looseItem,
        tree: looseTree,
      });
    }

    return projects;
  }

  function buildTreeFromFiles(baseDir: string, files: FileItem[]): TreeNode[] {
    const rootNodes: TreeNode[] = [];

    for (const file of files) {
      const rel = file.path.startsWith(baseDir)
        ? file.path.slice(baseDir.length).replace(/^\//, '')
        : file.name;

      const parts = rel.split('/');
      let currentLevel = rootNodes;
      let currentPath = baseDir;

      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const isFile = i === parts.length - 1;
        currentPath = currentPath ? `${currentPath}/${part}` : part;

        let existing = currentLevel.find((n) => n.name === part);
        if (!existing) {
          const folderItem: FileItem = {
            name: part,
            path: currentPath,
            is_dir: true,
            is_symlink: false,
            is_hidden: false,
            size_bytes: 0,
            formatted_size: '--',
            modified_timestamp: 0,
            formatted_modified: '--',
            permissions: 'rwxr-xr-x',
            extension: '',
          };
          existing = {
            name: part,
            path: currentPath,
            isDir: !isFile,
            item: isFile ? file : folderItem,
            children: [],
            sizeFormatted: isFile ? file.formatted_size : undefined,
            modifiedFormatted: isFile ? file.formatted_modified : undefined,
          };
          currentLevel.push(existing);
        }
        currentLevel = existing.children;
      }
    }

    sortTreeLevel(rootNodes);
    return rootNodes;
  }

  function sortTreeLevel(nodes: TreeNode[]) {
    nodes.sort((a, b) => {
      if (a.isDir && !b.isDir) return -1;
      if (!a.isDir && b.isDir) return 1;
      return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' });
    });
    for (const n of nodes) {
      if (n.children.length > 0) {
        sortTreeLevel(n.children);
      }
    }
  }

  // Hover handlers in Root component instance
  function onLocalFolderMouseEnter(path: string, folderItem?: FileItem) {
    if (collapsedByClick.has(path)) return;
    clearTimeout(hoverTimer);
    clearTimeout(hoverPreviewTimer);
    hoveredPath = path;

    // 1. Quick preview in Inspector (60ms) - reveals directory content / file stats
    if (folderItem) {
      hoverPreviewTimer = setTimeout(() => {
        if (hoveredPath === path) {
          activeHoveredItem.set(folderItem);
          onSelectPreview(folderItem);
        }
      }, 60);
    }

    // 2. Auto-expand folder tree inline (160ms) - seamless navigation without click fatigue
    hoverTimer = setTimeout(() => {
      if (hoveredPath === path) {
        localExpandedPaths.add(path);
        localExpandedPaths = new Set(localExpandedPaths);
      }
    }, 160);
  }

  function onLocalFolderMouseLeave() {
    clearTimeout(hoverTimer);
    clearTimeout(hoverPreviewTimer);
    hoveredPath = null;
  }

  function onLocalToggleFolder(path: string, e?: MouseEvent) {
    e?.stopPropagation();
    clearTimeout(hoverTimer);
    if (localExpandedPaths.has(path)) {
      localExpandedPaths.delete(path);
      collapsedByClick.add(path);
    } else {
      localExpandedPaths.add(path);
      collapsedByClick.delete(path);
    }
    localExpandedPaths = new Set(localExpandedPaths);
  }

  function toggleCategory(catType: string) {
    if (expandedCategoryTypes.has(catType)) {
      expandedCategoryTypes.delete(catType);
    } else {
      expandedCategoryTypes.add(catType);
    }
    expandedCategoryTypes = new Set(expandedCategoryTypes);
  }

  function onLocalFileMouseEnter(item?: FileItem) {
    if (!item) return;
    clearTimeout(hoverTimer);
    clearTimeout(hoverPreviewTimer);
    hoveredPath = item.path;

    hoverPreviewTimer = setTimeout(() => {
      if (hoveredPath === item.path) {
        activeHoveredItem.set(item);
        onSelectPreview(item);
      }
    }, 60);
  }

  function onLocalFileMouseLeave() {
    clearTimeout(hoverTimer);
    clearTimeout(hoverPreviewTimer);
    hoveredPath = null;
  }

  function getFileIconComponent(name: string) {
    const ext = name.split('.').pop()?.toLowerCase() || '';
    if (['rs', 'py', 'ts', 'js', 'c', 'cpp', 'swift', 'sh', 'bash', 'zsh', 'go', 'r'].includes(ext)) return FileCode;
    return FileText;
  }

  function getFileColor(name: string) {
    const ext = name.split('.').pop()?.toLowerCase() || '';
    if (ext === 'rs') return 'text-orange-400';
    if (ext === 'py') return 'text-yellow-400';
    if (['sh', 'bash', 'zsh'].includes(ext)) return 'text-cyan-400';
    if (['smk', 'snakefile'].includes(ext) || name.toLowerCase() === 'snakefile') return 'text-emerald-400';
    if (['toml', 'yaml', 'yml'].includes(ext)) return 'text-amber-400';
    if (['ts', 'js', 'json'].includes(ext)) return 'text-sky-400';
    return 'text-slate-300';
  }
</script>

{#if treeNodes}
  <!-- RECURSIVE NODE RENDERING (Sub-directories and files) -->
  <div class="flex flex-col space-y-0.5">
    {#each treeNodes as node (node.path)}
      {@const isOpen = expandedPaths.has(node.path)}
      {#if node.isDir}
        <div>
          <!-- Folder Row -->
          <div
            class="group flex items-center justify-between px-2 py-1 rounded hover:bg-[var(--bg-hover)] transition-colors cursor-pointer {isOpen ? 'bg-[var(--bg-hover)]/40 text-[var(--text-primary)]' : 'text-[var(--text-secondary)]'}"
            on:mouseenter={() => handleFolderMouseEnter(node.path, node.item)}
            on:mouseleave={handleFolderMouseLeave}
            on:click={(e) => toggleFolder(node.path, e)}
            on:dblclick={() => onNavigateFolder(node.path)}
            role="treeitem"
            aria-expanded={isOpen}
            tabindex="-1"
          >
            <div class="flex items-center gap-1.5 min-w-0 flex-1">
              <button
                type="button"
                class="text-slate-400 hover:text-amber-400 w-3.5 flex justify-center shrink-0 cursor-pointer"
                on:click|stopPropagation={(e) => toggleFolder(node.path, e)}
                on:mouseenter|stopPropagation={() => handleFolderMouseEnter(node.path, node.item)}
                title={isOpen ? 'Klicka för att fälla ihop' : 'Klicka eller hovra för att fälla ut'}
              >
                {#if isOpen}
                  <ChevronDown size={12} class="text-[var(--accent)]" />
                {:else}
                  <ChevronRight size={12} />
                {/if}
              </button>
              <Folder size={13} class="text-amber-400/90 shrink-0" />
              <span class="font-sans font-medium text-xs truncate">{node.name}</span>
            </div>

            <!-- Action buttons on hover -->
            <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 shrink-0 ml-1">
              <button
                class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                on:click|stopPropagation={() => onNavigateFolder(node.path)}
                title="Öppna mappen i fillistan och stäng index"
              >
                <FolderOpen size={11} />
              </button>
              <button
                class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                on:click|stopPropagation={() => revealInOs(node.path)}
                title="Visa i Finder"
              >
                <ExternalLink size={11} />
              </button>
            </div>
          </div>

          <!-- Indented Sub-Children -->
          {#if isOpen && node.children.length > 0}
            <div class="ml-3 pl-2 border-l border-[var(--border)]/60 my-0.5 space-y-0.5">
              <svelte:self
                treeNodes={node.children}
                {onSelectPreview}
                {onNavigateFolder}
                {expandedPaths}
                {toggleFolder}
                {handleFolderMouseEnter}
                {handleFolderMouseLeave}
                {handleFileHover}
                {handleFileMouseLeave}
              />
            </div>
          {/if}
        </div>
      {:else}
        <!-- File Row -->
        <div
          class="group flex items-center justify-between px-2 py-1 rounded hover:bg-[var(--bg-hover)] transition-colors cursor-pointer text-[var(--text-primary)]"
          on:mouseenter={() => handleFileHover(node.item)}
          on:mouseleave={handleFileMouseLeave}
          on:click={() => node.item && onSelectPreview(node.item)}
          on:dblclick={() => node.item && openInFavoriteEditor(node.item.path)}
          role="row"
          tabindex="-1"
        >
          <div class="flex items-center gap-1.5 min-w-0 flex-1 ml-3.5">
            <svelte:component this={getFileIconComponent(node.name)} size={12} class="{getFileColor(node.name)} shrink-0" />
            <span class="font-sans text-xs truncate">{node.name}</span>
          </div>

          <div class="flex items-center gap-2 shrink-0 ml-2">
            {#if node.sizeFormatted}
              <span class="text-[10px] text-[var(--text-muted)] font-mono">{node.sizeFormatted}</span>
            {/if}

            <!-- Action buttons on hover -->
            <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100">
              <button
                class="p-1 rounded hover:bg-sky-600 hover:text-white text-slate-400 transition-colors cursor-pointer"
                on:click|stopPropagation={() => openInFavoriteEditor(node.path)}
                title="Öppna i {$favoriteEditor} (⌘E)"
              >
                <FileCode size={11} />
              </button>
              <button
                class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                on:click|stopPropagation={() => revealInOs(node.path)}
                title="Visa i Finder"
              >
                <ExternalLink size={11} />
              </button>
            </div>
          </div>
        </div>
      {/if}
    {/each}
  </div>
{:else}
  <!-- ROOT LEVEL TREE VIEW (Categorized Projects) -->
  <div class="flex-1 min-h-0 w-full bg-[var(--bg-base)] overflow-y-auto text-xs font-mono select-none p-3 pb-16 space-y-3.5" style="overscroll-behavior: contain; -webkit-overflow-scrolling: touch;">
    {#if detectedProjects.length === 0}
      <div class="p-12 text-center text-[var(--text-muted)] flex flex-col items-center justify-center space-y-2">
        <Folder size={32} class="opacity-20" />
        <span>Inga kodprojekt eller källkodsfiler hittades i mappen.</span>
      </div>
    {/if}

    <!-- 1. Rust Projects Section -->
    {#if categorizedProjects.rust.length > 0}
      {@const isCatOpen = expandedCategoryTypes.has('rust')}
      <div class="border border-[var(--border)] rounded-xl bg-[var(--bg-surface)]/90 overflow-hidden shadow-sm">
        <button
          class="w-full flex items-center justify-between px-3.5 py-2 bg-[var(--bg-panel)]/90 hover:bg-[var(--bg-hover)] text-left transition-colors cursor-pointer border-b border-[var(--border)]/60"
          on:click={() => toggleCategory('rust')}
        >
          <div class="flex items-center gap-2 font-sans font-bold text-sm text-[var(--text-primary)]">
            <Cpu size={15} class="text-orange-400" />
            <span>Rust-projekt</span>
            <span class="text-[11px] px-2 py-0.2 rounded-full bg-orange-500/20 text-orange-300 font-mono font-semibold border border-orange-500/30">
              {categorizedProjects.rust.length} {categorizedProjects.rust.length === 1 ? 'projekt' : 'projekt'}
            </span>
          </div>
          {#if isCatOpen}
            <ChevronDown size={14} class="text-[var(--text-muted)]" />
          {:else}
            <ChevronRight size={14} class="text-[var(--text-muted)]" />
          {/if}
        </button>

        {#if isCatOpen}
          <div class="p-2 space-y-1.5 divide-y divide-[var(--border)]/30">
            {#each categorizedProjects.rust as project (project.id)}
              {@const isProjectOpen = localExpandedPaths.has(project.path)}
              <div class="pt-1.5 first:pt-0">
                <!-- Project Root Header -->
                <div
                  class="group flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-[var(--bg-hover)] transition-colors cursor-pointer {isProjectOpen ? 'bg-[var(--bg-hover)]/70' : ''}"
                  on:mouseenter={() => onLocalFolderMouseEnter(project.path, project.item)}
                  on:mouseleave={onLocalFolderMouseLeave}
                  on:click={(e) => onLocalToggleFolder(project.path, e)}
                  on:dblclick={() => onNavigateFolder(project.path)}
                  role="treeitem"
                  aria-expanded={isProjectOpen}
                  tabindex="-1"
                >
                  <div class="flex items-center gap-2 min-w-0 flex-1">
                    <button
                      type="button"
                      class="text-slate-400 hover:text-amber-400 w-3.5 flex justify-center shrink-0 cursor-pointer"
                      on:click|stopPropagation={(e) => onLocalToggleFolder(project.path, e)}
                      on:mouseenter|stopPropagation={() => onLocalFolderMouseEnter(project.path, project.item)}
                      title={isProjectOpen ? 'Klicka för att fälla ihop' : 'Klicka eller hovra för att fälla ut'}
                    >
                      {#if isProjectOpen}
                        <ChevronDown size={13} class="text-[var(--accent)]" />
                      {:else}
                        <ChevronRight size={13} />
                      {/if}
                    </button>
                    <Folder size={15} class="text-amber-400 shrink-0" />
                    <span class="font-sans font-bold text-sm text-[var(--text-primary)] truncate">{project.name}</span>
                    <span class="text-[9.5px] font-mono px-1.5 py-0.2 rounded bg-orange-500/20 text-orange-300 border border-orange-500/30 shrink-0">
                      Cargo.toml
                    </span>
                    <span class="text-[10px] text-[var(--text-muted)] font-mono truncate hidden sm:inline" title={project.relativePath}>
                      {project.relativePath}
                    </span>
                  </div>

                  <!-- Action buttons on hover -->
                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 shrink-0 ml-2">
                    <button
                      class="flex items-center gap-1 px-2.5 py-1 rounded bg-sky-600/20 hover:bg-sky-600 text-sky-300 hover:text-white border border-sky-500/40 text-[11px] font-sans font-semibold transition-colors cursor-pointer"
                      on:click|stopPropagation={() => openInFavoriteEditor(project.path)}
                      title="Öppna hela projektet i {$favoriteEditor} (⌘E)"
                    >
                      <FileCode size={12} />
                      <span>Öppna i {$favoriteEditor}</span>
                    </button>
                    <button
                      class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                      on:click|stopPropagation={() => onNavigateFolder(project.path)}
                      title="Öppna mappen i fillistan och stäng index"
                    >
                      <FolderOpen size={13} />
                    </button>
                    <button
                      class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                      on:click|stopPropagation={() => revealInOs(project.path)}
                      title="Visa i Finder"
                    >
                      <ExternalLink size={12} />
                    </button>
                  </div>
                </div>

                <!-- Nested Tree -->
                {#if isProjectOpen}
                  <div class="ml-4 pl-2.5 my-1 border-l-2 border-[var(--border)]/70 space-y-0.5">
                    <svelte:self
                      treeNodes={project.tree}
                      {onSelectPreview}
                      {onNavigateFolder}
                      expandedPaths={localExpandedPaths}
                      toggleFolder={onLocalToggleFolder}
                      handleFolderMouseEnter={onLocalFolderMouseEnter}
                      handleFolderMouseLeave={onLocalFolderMouseLeave}
                      handleFileHover={onLocalFileMouseEnter}
                      handleFileMouseLeave={onLocalFileMouseLeave}
                    />
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- 2. Pipelines Section (Snakemake / Nextflow) -->
    {#if categorizedProjects.pipeline.length > 0}
      {@const isCatOpen = expandedCategoryTypes.has('pipeline')}
      <div class="border border-[var(--border)] rounded-xl bg-[var(--bg-surface)]/90 overflow-hidden shadow-sm">
        <button
          class="w-full flex items-center justify-between px-3.5 py-2 bg-[var(--bg-panel)]/90 hover:bg-[var(--bg-hover)] text-left transition-colors cursor-pointer border-b border-[var(--border)]/60"
          on:click={() => toggleCategory('pipeline')}
        >
          <div class="flex items-center gap-2 font-sans font-bold text-sm text-[var(--text-primary)]">
            <GitBranch size={15} class="text-emerald-400" />
            <span>Pipelines & Workflow</span>
            <span class="text-[11px] px-2 py-0.2 rounded-full bg-emerald-500/20 text-emerald-300 font-mono font-semibold border border-emerald-500/30">
              {categorizedProjects.pipeline.length} pipelines
            </span>
          </div>
          {#if isCatOpen}
            <ChevronDown size={14} class="text-[var(--text-muted)]" />
          {:else}
            <ChevronRight size={14} class="text-[var(--text-muted)]" />
          {/if}
        </button>

        {#if isCatOpen}
          <div class="p-2 space-y-1.5 divide-y divide-[var(--border)]/30">
            {#each categorizedProjects.pipeline as project (project.id)}
              {@const isProjectOpen = localExpandedPaths.has(project.path)}
              <div class="pt-1.5 first:pt-0">
                <div
                  class="group flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-[var(--bg-hover)] transition-colors cursor-pointer {isProjectOpen ? 'bg-[var(--bg-hover)]/70' : ''}"
                  on:mouseenter={() => onLocalFolderMouseEnter(project.path, project.item)}
                  on:mouseleave={onLocalFolderMouseLeave}
                  on:click={(e) => onLocalToggleFolder(project.path, e)}
                  on:dblclick={() => onNavigateFolder(project.path)}
                  role="treeitem"
                  aria-expanded={isProjectOpen}
                  tabindex="-1"
                >
                  <div class="flex items-center gap-2 min-w-0 flex-1">
                    <button
                      type="button"
                      class="text-slate-400 hover:text-emerald-400 w-3.5 flex justify-center shrink-0 cursor-pointer"
                      on:click|stopPropagation={(e) => onLocalToggleFolder(project.path, e)}
                      on:mouseenter|stopPropagation={() => onLocalFolderMouseEnter(project.path, project.item)}
                      title={isProjectOpen ? 'Klicka för att fälla ihop' : 'Klicka eller hovra för att fälla ut'}
                    >
                      {#if isProjectOpen}
                        <ChevronDown size={13} class="text-emerald-400" />
                      {:else}
                        <ChevronRight size={13} />
                      {/if}
                    </button>
                    <Folder size={15} class="text-emerald-400 shrink-0" />
                    <span class="font-sans font-bold text-sm text-[var(--text-primary)] truncate">{project.name}</span>
                    <span class="text-[9.5px] font-mono px-1.5 py-0.2 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 shrink-0">
                      {project.manifest}
                    </span>
                    <span class="text-[10px] text-[var(--text-muted)] font-mono truncate hidden sm:inline" title={project.relativePath}>
                      {project.relativePath}
                    </span>
                  </div>

                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 shrink-0 ml-2">
                    <button
                      class="flex items-center gap-1 px-2.5 py-1 rounded bg-sky-600/20 hover:bg-sky-600 text-sky-300 hover:text-white border border-sky-500/40 text-[11px] font-sans font-semibold transition-colors cursor-pointer"
                      on:click|stopPropagation={() => openInFavoriteEditor(project.path)}
                      title="Öppna hela projektet i {$favoriteEditor} (⌘E)"
                    >
                      <FileCode size={12} />
                      <span>Öppna i {$favoriteEditor}</span>
                    </button>
                    <button
                      class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                      on:click|stopPropagation={() => onNavigateFolder(project.path)}
                      title="Öppna mappen i fillistan och stäng index"
                    >
                      <FolderOpen size={13} />
                    </button>
                    <button
                      class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                      on:click|stopPropagation={() => revealInOs(project.path)}
                      title="Visa i Finder"
                    >
                      <ExternalLink size={12} />
                    </button>
                  </div>
                </div>

                {#if isProjectOpen}
                  <div class="ml-4 pl-2.5 my-1 border-l-2 border-[var(--border)]/70 space-y-0.5">
                    <svelte:self
                      treeNodes={project.tree}
                      {onSelectPreview}
                      {onNavigateFolder}
                      expandedPaths={localExpandedPaths}
                      toggleFolder={onLocalToggleFolder}
                      handleFolderMouseEnter={onLocalFolderMouseEnter}
                      handleFolderMouseLeave={onLocalFolderMouseLeave}
                      handleFileHover={onLocalFileMouseEnter}
                      handleFileMouseLeave={onLocalFileMouseLeave}
                    />
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- 3. Python Section -->
    {#if categorizedProjects.python.length > 0}
      {@const isCatOpen = expandedCategoryTypes.has('python')}
      <div class="border border-[var(--border)] rounded-xl bg-[var(--bg-surface)]/90 overflow-hidden shadow-sm">
        <button
          class="w-full flex items-center justify-between px-3.5 py-2 bg-[var(--bg-panel)]/90 hover:bg-[var(--bg-hover)] text-left transition-colors cursor-pointer border-b border-[var(--border)]/60"
          on:click={() => toggleCategory('python')}
        >
          <div class="flex items-center gap-2 font-sans font-bold text-sm text-[var(--text-primary)]">
            <FileCode size={15} class="text-yellow-400" />
            <span>Python & Notebooks</span>
            <span class="text-[11px] px-2 py-0.2 rounded-full bg-yellow-500/20 text-yellow-300 font-mono font-semibold border border-yellow-500/30">
              {categorizedProjects.python.length} projekt
            </span>
          </div>
          {#if isCatOpen}
            <ChevronDown size={14} class="text-[var(--text-muted)]" />
          {:else}
            <ChevronRight size={14} class="text-[var(--text-muted)]" />
          {/if}
        </button>

        {#if isCatOpen}
          <div class="p-2 space-y-1.5 divide-y divide-[var(--border)]/30">
            {#each categorizedProjects.python as project (project.id)}
              {@const isProjectOpen = localExpandedPaths.has(project.path)}
              <div class="pt-1.5 first:pt-0">
                <div
                  class="group flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-[var(--bg-hover)] transition-colors cursor-pointer {isProjectOpen ? 'bg-[var(--bg-hover)]/70' : ''}"
                  on:mouseenter={() => onLocalFolderMouseEnter(project.path, project.item)}
                  on:mouseleave={onLocalFolderMouseLeave}
                  on:click={(e) => onLocalToggleFolder(project.path, e)}
                  on:dblclick={() => onNavigateFolder(project.path)}
                  role="treeitem"
                  aria-expanded={isProjectOpen}
                  tabindex="-1"
                >
                  <div class="flex items-center gap-2 min-w-0 flex-1">
                    <button
                      type="button"
                      class="text-slate-400 hover:text-yellow-400 w-3.5 flex justify-center shrink-0 cursor-pointer"
                      on:click|stopPropagation={(e) => onLocalToggleFolder(project.path, e)}
                      on:mouseenter|stopPropagation={() => onLocalFolderMouseEnter(project.path, project.item)}
                      title={isProjectOpen ? 'Klicka för att fälla ihop' : 'Klicka eller hovra för att fälla ut'}
                    >
                      {#if isProjectOpen}
                        <ChevronDown size={13} class="text-yellow-400" />
                      {:else}
                        <ChevronRight size={13} />
                      {/if}
                    </button>
                    <Folder size={15} class="text-yellow-400 shrink-0" />
                    <span class="font-sans font-bold text-sm text-[var(--text-primary)] truncate">{project.name}</span>
                    <span class="text-[9.5px] font-mono px-1.5 py-0.2 rounded bg-yellow-500/20 text-yellow-300 border border-yellow-500/30 shrink-0">
                      {project.manifest}
                    </span>
                    <span class="text-[10px] text-[var(--text-muted)] font-mono truncate hidden sm:inline" title={project.relativePath}>
                      {project.relativePath}
                    </span>
                  </div>

                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 shrink-0 ml-2">
                    <button
                      class="flex items-center gap-1 px-2.5 py-1 rounded bg-sky-600/20 hover:bg-sky-600 text-sky-300 hover:text-white border border-sky-500/40 text-[11px] font-sans font-semibold transition-colors cursor-pointer"
                      on:click|stopPropagation={() => openInFavoriteEditor(project.path)}
                      title="Öppna hela projektet i {$favoriteEditor} (⌘E)"
                    >
                      <FileCode size={12} />
                      <span>Öppna i {$favoriteEditor}</span>
                    </button>
                    <button
                      class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                      on:click|stopPropagation={() => onNavigateFolder(project.path)}
                      title="Öppna mappen i fillistan och stäng index"
                    >
                      <FolderOpen size={13} />
                    </button>
                    <button
                      class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                      on:click|stopPropagation={() => revealInOs(project.path)}
                      title="Visa i Finder"
                    >
                      <ExternalLink size={12} />
                    </button>
                  </div>
                </div>

                {#if isProjectOpen}
                  <div class="ml-4 pl-2.5 my-1 border-l-2 border-[var(--border)]/70 space-y-0.5">
                    <svelte:self
                      treeNodes={project.tree}
                      {onSelectPreview}
                      {onNavigateFolder}
                      expandedPaths={localExpandedPaths}
                      toggleFolder={onLocalToggleFolder}
                      handleFolderMouseEnter={onLocalFolderMouseEnter}
                      handleFolderMouseLeave={onLocalFolderMouseLeave}
                      handleFileHover={onLocalFileMouseEnter}
                      handleFileMouseLeave={onLocalFileMouseLeave}
                    />
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- 4. Web & TypeScript Section -->
    {#if categorizedProjects.web.length > 0}
      {@const isCatOpen = expandedCategoryTypes.has('web')}
      <div class="border border-[var(--border)] rounded-xl bg-[var(--bg-surface)]/90 overflow-hidden shadow-sm">
        <button
          class="w-full flex items-center justify-between px-3.5 py-2 bg-[var(--bg-panel)]/90 hover:bg-[var(--bg-hover)] text-left transition-colors cursor-pointer border-b border-[var(--border)]/60"
          on:click={() => toggleCategory('web')}
        >
          <div class="flex items-center gap-2 font-sans font-bold text-sm text-[var(--text-primary)]">
            <Globe size={15} class="text-sky-400" />
            <span>TypeScript & Web</span>
            <span class="text-[11px] px-2 py-0.2 rounded-full bg-sky-500/20 text-sky-300 font-mono font-semibold border border-sky-500/30">
              {categorizedProjects.web.length} projekt
            </span>
          </div>
          {#if isCatOpen}
            <ChevronDown size={14} class="text-[var(--text-muted)]" />
          {:else}
            <ChevronRight size={14} class="text-[var(--text-muted)]" />
          {/if}
        </button>

        {#if isCatOpen}
          <div class="p-2 space-y-1.5 divide-y divide-[var(--border)]/30">
            {#each categorizedProjects.web as project (project.id)}
              {@const isProjectOpen = localExpandedPaths.has(project.path)}
              <div class="pt-1.5 first:pt-0">
                <div
                  class="group flex items-center justify-between px-2.5 py-1.5 rounded-lg hover:bg-[var(--bg-hover)] transition-colors cursor-pointer {isProjectOpen ? 'bg-[var(--bg-hover)]/70' : ''}"
                  on:mouseenter={() => onLocalFolderMouseEnter(project.path, project.item)}
                  on:mouseleave={onLocalFolderMouseLeave}
                  on:click={(e) => onLocalToggleFolder(project.path, e)}
                  on:dblclick={() => onNavigateFolder(project.path)}
                  role="treeitem"
                  aria-expanded={isProjectOpen}
                  tabindex="-1"
                >
                  <div class="flex items-center gap-2 min-w-0 flex-1">
                    <button
                      type="button"
                      class="text-slate-400 hover:text-sky-400 w-3.5 flex justify-center shrink-0 cursor-pointer"
                      on:click|stopPropagation={(e) => onLocalToggleFolder(project.path, e)}
                      on:mouseenter|stopPropagation={() => onLocalFolderMouseEnter(project.path, project.item)}
                      title={isProjectOpen ? 'Klicka för att fälla ihop' : 'Klicka eller hovra för att fälla ut'}
                    >
                      {#if isProjectOpen}
                        <ChevronDown size={13} class="text-sky-400" />
                      {:else}
                        <ChevronRight size={13} />
                      {/if}
                    </button>
                    <Folder size={15} class="text-sky-400 shrink-0" />
                    <span class="font-sans font-bold text-sm text-[var(--text-primary)] truncate">{project.name}</span>
                    <span class="text-[9.5px] font-mono px-1.5 py-0.2 rounded bg-sky-500/20 text-sky-300 border border-sky-500/30 shrink-0">
                      package.json
                    </span>
                    <span class="text-[10px] text-[var(--text-muted)] font-mono truncate hidden sm:inline" title={project.relativePath}>
                      {project.relativePath}
                    </span>
                  </div>

                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 shrink-0 ml-2">
                    <button
                      class="flex items-center gap-1 px-2.5 py-1 rounded bg-sky-600/20 hover:bg-sky-600 text-sky-300 hover:text-white border border-sky-500/40 text-[11px] font-sans font-semibold transition-colors cursor-pointer"
                      on:click|stopPropagation={() => openInFavoriteEditor(project.path)}
                      title="Öppna hela projektet i {$favoriteEditor} (⌘E)"
                    >
                      <FileCode size={12} />
                      <span>Öppna i {$favoriteEditor}</span>
                    </button>
                    <button
                      class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                      on:click|stopPropagation={() => onNavigateFolder(project.path)}
                      title="Öppna mappen i fillistan och stäng index"
                    >
                      <FolderOpen size={13} />
                    </button>
                    <button
                      class="p-1 rounded hover:bg-[var(--bg-panel)] text-slate-400 hover:text-white transition-colors cursor-pointer"
                      on:click|stopPropagation={() => revealInOs(project.path)}
                      title="Visa i Finder"
                    >
                      <ExternalLink size={12} />
                    </button>
                  </div>
                </div>

                {#if isProjectOpen}
                  <div class="ml-4 pl-2.5 my-1 border-l-2 border-[var(--border)]/70 space-y-0.5">
                    <svelte:self
                      treeNodes={project.tree}
                      {onSelectPreview}
                      {onNavigateFolder}
                      expandedPaths={localExpandedPaths}
                      toggleFolder={onLocalToggleFolder}
                      handleFolderMouseEnter={onLocalFolderMouseEnter}
                      handleFolderMouseLeave={onLocalFolderMouseLeave}
                      handleFileHover={onLocalFileMouseEnter}
                      handleFileMouseLeave={onLocalFileMouseLeave}
                    />
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- 5. Loose Scripts Section -->
    {#if categorizedProjects.script.length > 0}
      {@const isCatOpen = expandedCategoryTypes.has('script')}
      {@const scriptProj = categorizedProjects.script[0]}
      <div class="border border-[var(--border)] rounded-xl bg-[var(--bg-surface)]/90 overflow-hidden shadow-sm">
        <button
          class="w-full flex items-center justify-between px-3.5 py-2 bg-[var(--bg-panel)]/90 hover:bg-[var(--bg-hover)] text-left transition-colors cursor-pointer border-b border-[var(--border)]/60"
          on:click={() => toggleCategory('script')}
        >
          <div class="flex items-center gap-2 font-sans font-bold text-sm text-[var(--text-primary)]">
            <Terminal size={15} class="text-cyan-400" />
            <span>Fristående skript & verktyg</span>
            <span class="text-[11px] px-2 py-0.2 rounded-full bg-cyan-500/20 text-cyan-300 font-mono font-semibold border border-cyan-500/30">
              {scriptProj.fileCount} skript
            </span>
          </div>
          {#if isCatOpen}
            <ChevronDown size={14} class="text-[var(--text-muted)]" />
          {:else}
            <ChevronRight size={14} class="text-[var(--text-muted)]" />
          {/if}
        </button>

        {#if isCatOpen}
          <div class="p-2">
            <svelte:self
              treeNodes={scriptProj.tree}
              {onSelectPreview}
              {onNavigateFolder}
              expandedPaths={localExpandedPaths}
              toggleFolder={onLocalToggleFolder}
              handleFolderMouseEnter={onLocalFolderMouseEnter}
              handleFolderMouseLeave={onLocalFolderMouseLeave}
              handleFileHover={onLocalFileMouseEnter}
              handleFileMouseLeave={onLocalFileMouseLeave}
            />
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}
