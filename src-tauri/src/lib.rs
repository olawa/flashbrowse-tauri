pub mod bio_commands;
pub mod fs_commands;
pub mod models;
pub mod preview_commands;
pub mod ssh_commands;
pub mod terminal_commands;

use bio_commands::*;
use fs_commands::*;
use preview_commands::*;
use ssh_commands::*;
use terminal_commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_home_directory,
            list_directory,
            get_disk_info,
            calculate_dir_size,
            trash_items,
            copy_items,
            transfer_items,
            move_items,
            create_directory,
            create_file,
            rename_item,
            open_in_default,
            open_file_with,
            reveal_in_os,
            get_preview,
            run_command,
            tab_complete,
            ssh_list_directory,
            ssh_get_preview,
            ssh_run_command,
            ssh_open_file_locally,
            quick_look,
            toggle_detached_inspector,
            get_bam_header,
            generate_rsnap_snapshot,
            launch_rsnap,
            run_rs_qc,
            list_archive_contents,
            scan_directory_index,
            get_bam_alignments,
            get_directory_notes,
            save_directory_notes,
            create_zip_archive,
            watch_directory,
            deep_search,
            start_rsnap_server,
            stop_rsnap_server,
            get_rsnap_server_status,
            send_to_igv,
            check_igv_status,
            get_configured_genomes,
            save_configured_genome,
            detect_track_genomes,
            get_subdirs_tree
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
