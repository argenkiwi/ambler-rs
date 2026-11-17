# Program Specifications

This program downloads audio files specified in an M3U playlist. It takes an M3U file path as a command-line argument, optionally resolves `khinsider.com` URLs to direct download links, then downloads all files to a local folder.

## Nodes

### Check M3u File
- Initial node of the application.
- Reads the file path from the first command-line argument.
- If no path is provided, or if the path does not have a `.m3u` extension, or if the file does not exist or is not a regular file, it prints an error and terminates.
- If valid, it stores the path in state and transitions to `READ_M3U_FILE`.

### Read M3u File
- Reads the M3U file at the stored path.
- Parses each line, ignoring empty lines and comment lines (starting with `#`).
- Stores the extracted URLs in state and prints them.
- If any URL starts with `https://downloads.khinsider.com/game-soundtracks`, it transitions to `PROMPT_RESOLVE`.
- Otherwise, it transitions to `PROMPT_DOWNLOAD`.

### Prompt Resolve
- Displays: `Some URLs require resolution. Proceed? (y/n)`.
- Loops until the user enters `y`/`yes` or `n`/`no`.
- If `y`/`yes`, it transitions to `RESOLVE_URLS`.
- If `n`/`no`, it terminates.

### Resolve Urls
- For each URL in state: if it is a khinsider URL, resolves it to the direct download link; otherwise keeps it as-is.
- All resolutions run in parallel.
- Updates state with the resolved URLs and transitions to `SAVE_M3U_FILE`.

### Save M3u File
- Joins the URLs with newlines and overwrites the M3U file at the stored path.
- Prints the saved file path and the current URL list.
- Transitions to `PROMPT_DOWNLOAD`.

### Prompt Download
- Displays: `Proceed with download? (y/n)`.
- Loops until the user enters `y`/`yes` or `n`/`no`.
- If `y`/`yes`, it transitions to `DOWNLOAD_FILES`.
- If `n`/`no`, it terminates.

### Download Files
- Derives the output folder name from the M3U filename (without extension).
- Downloads all URLs to the output folder in parallel.
- Removes the original M3U file.
- Updates state with the M3U file path set to `<outputFolder>/playlist.m3u` and the URLs replaced with `file://` URIs of the downloaded files.
- Transitions to `SAVE_LOCAL_M3U_FILE`.

### Save Local M3u File
- The same node as `SAVE_M3U_FILE`, wired under a second ID because its edge differs.
- Saves the local playlist, prints the saved file path and the URL list, and terminates.

## Shared State

The shared state consists of:
- `m3u_file_path`: The path to the M3U file being processed.
- `urls`: The list of URLs extracted from or written to the M3U file.
