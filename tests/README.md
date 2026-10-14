# Regression tests

Run `npm test` with Node.js 22.6 or newer (the runner uses native TypeScript
stripping). Run backend tests with `cd src-tauri && cargo test --lib --offline`.

The frontend tests exercise notes autosave with delayed and failed writes, run
the actual inspector loading functions with delayed IPC responses, and check
notebook sanitization and HTML sandbox/scroll behavior. They do not launch a
native Tauri webview.

Genomics tests cover FASTQ pair grouping, sample provenance, multi-sample VCFs,
and reference comparisons with incomplete metadata. Rust tests also exercise
index status and the explicit quickcheck/index creation flow using samtools when
it is installed.

The backend tests run the remote filename-selection shell script locally in
temporary directories, reject remote trash operations on matching local paths,
and use a noisy child process to check both transfer backends. They do not
connect to an SSH server.
