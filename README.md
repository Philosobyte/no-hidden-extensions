<img src="resources/tray_and_windows_icon.png" width="140px" />

# no-hidden-extensions

An application for making sure that Windows Explorer file extensions are never hidden. 
It runs at startup and stays minimized to the system tray. When something turns file extension hiding back on, 
`no-hidden-extensions` pops up and notifies the user, allowing you to turn file extension hiding back off.

# Supported platforms
This has currently only been tested with Windows 10. 

# Future work
- 
- test on Windows 11
- add support for detecting possibly malicious file extensions, including usage of the Unicode right-to-left mark
  - this will require a kernel driver