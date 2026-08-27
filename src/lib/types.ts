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
  kind: 'code' | 'text' | 'image' | 'table' | 'hex' | 'too_large' | 'error';
  text_content?: string;
  language?: string;
  line_count?: number;
  image_base64?: string;
  image_mime?: string;
  table_headers?: string[];
  table_rows?: string[][];
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

export type ThemeName = 'pro-dark' | 'cyberpunk' | 'retro-mac' | 'kids-mode';
