export interface FileItem {
  name: string;
  path: string;
  is_dir: boolean;
  is_symlink: boolean;
  size_bytes: number;
  formatted_size: string;
  modified_timestamp: number;
  formatted_modified: string;
  extension: string;
  is_hidden: boolean;
  permissions: string;
  item_count?: number;
}

export interface DiskInfo {
  mount_point: string;
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  formatted_total: string;
  formatted_available: string;
  formatted_used: string;
  percentage_used: number;
}

export interface PreviewContent {
  kind: 'code' | 'text' | 'html' | 'pdf' | 'markdown' | 'image' | 'svg' | 'video' | 'audio' | 'table' | 'notebook' | 'hex' | 'too_large' | 'error' | 'directory';
  text_content?: string;
  html_content?: string;
  pdf_base64?: string;
  media_base64?: string;
  media_mime?: string;
  language?: string;
  language_name?: string;
  language_emoji?: string;
  line_count?: number;
  image_base64?: string;
  image_mime?: string;
  table_headers?: string[];
  table_rows?: string[][];
  sheet_names?: string[];
  hex_lines?: string[];
  file_size_bytes: number;
  formatted_size: string;
  modified_str: string;
  permissions_str: string;
  error_message?: string;
}

export interface TerminalOutput {
  stdout: string;
  stderr: string;
  exit_code: number;
  new_cwd?: string;
}

export interface TabCompletionResult {
  completed_line: string;
  suggestions: string[];
}

export interface DirectorySummary {
  path: string;
  total_items: number;
  total_dirs: number;
  total_files: number;
  total_size_bytes: number;
  formatted_total_size: string;
}

export interface ContigInfo {
  name: string;
  length: number;
  formatted_length: string;
  assembly?: string;
}

export interface ReadGroupInfo {
  id: string;
  sample?: string;
  platform?: string;
  library?: string;
  center?: string;
}

export interface ProgramInfo {
  id: string;
  name?: string;
  version?: string;
  command_line?: string;
}

export interface BamHeaderData {
  detected_reference: string;
  reference_matched_path?: string;
  contigs: ContigInfo[];
  total_contigs: number;
  total_genome_length: number;
  formatted_genome_length: string;
  read_groups: ReadGroupInfo[];
  programs: ProgramInfo[];
  raw_header: string;
  has_index: boolean;
  index_type?: string;
}

export interface ArchiveEntry {
  name: string;
  size_bytes: number;
  formatted_size: string;
  is_dir: boolean;
  modified_str: string;
}

export interface ArchiveSummary {
  path: string;
  entries: ArchiveEntry[];
  total_files: number;
  total_uncompressed_bytes: number;
  formatted_uncompressed_size: string;
}

export interface DirectoryIndexGroup {
  directory_path: string;
  directory_name: string;
  relative_path: string;
  items: FileItem[];
}

export interface FileTypeIndexMeta {
  id: string;
  name: string;
  extensions: string[];
  badge: string;
  iconName: string;
  colorClass: string;
}

export type ThemeName = 'pro-dark' | 'cyberpunk' | 'retro-mac' | 'kids-mode';

