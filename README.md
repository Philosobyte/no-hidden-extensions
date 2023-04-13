<img src="resources/tray_and_window_icon.png" width="140"  />

# no-hidden-extensions

An application for making sure that Windows Explorer file extensions are never hidden. 
It runs at startup and stays minimized to the system tray. When something turns file extension hiding back on, 
`no-hidden-extensions` pops up and notifies the user, allowing you to turn file extension hiding back off.

<img src="https://i.postimg.cc/6QBBk4Bw/no-hidden-files-screenshot-extensions-hidden.png" width="475" />
<img src="https://i.postimg.cc/5tpfb9tz/no-hidden-files-screenshot-extensions-visible.png" width="475" />

# Typical Usage
1. Download the executable and verify its hash.
2. Open the executable.
3. Check the "Run at Windows startup" box.
4. If the "Stop hiding file extensions and restart Windows Explorer" button is not greyed out, click it.
5. Minimize and forget about it.

To test whether notification works on your system, unhide file extensions in Windows Explorer.
`no-hidden-extensions` should immediately pop up and its button should be clickable. 

# Building from source
No local dependencies are required to build this from source; just run `cargo build --release`

# Supported platforms
This has currently only been tested with Windows 10.

# Planned future work
- test on Windows 11
- add support for detecting possibly malicious file extensions, including usage of the Unicode right-to-left mark
