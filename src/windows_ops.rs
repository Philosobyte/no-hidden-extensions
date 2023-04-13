use std::borrow::Cow;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{Error, Result};
use tracing::{instrument, trace};
use windows_sys::Win32::Foundation::{BOOL, HANDLE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};
use winreg::{HKEY, RegKey};
use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_NOTIFY_CHANGE_LAST_SET};
use winreg::transaction::Transaction;
use winreg::types::{FromRegValue, ToRegValue};

use crate::err;

// Path to the registry key containing the value for hiding file extensions.
const WINDOWS_EXPLORER_REGKEY_SUBPATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced";

// The registry value under `WINDOWS_EXPLORER_REGKEY_SUBPATH` responsible for hiding file extensions.
const HIDE_FILE_EXT_VALUE_NAME: &str = "HideFileExt";

// Path to the registry key for registering applications which should run on Windows startup.
const WINDOWS_STARTUP_REGKEY_SUBPATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

// The name of this application as it should be known by the Windows Registry.
// Let's just use a hardcoded string to avoid multiple of this program from running at once.
const WINDOWS_STARTUP_VALUE_NAME: &str = "NoHiddenExtensions";

// Checks whether the currently running program will run on Windows startup.
// This is sensitive to the executable file being moved.
#[instrument]
pub(crate) fn will_app_run_at_startup() -> Result<bool> {
    let hive: RegKey = RegKey::predef(HKEY_CURRENT_USER);
    let run_on_startup_key: RegKey = hive.open_subkey(WINDOWS_STARTUP_REGKEY_SUBPATH)?;

    return match run_on_startup_key.get_value::<String, &str>(WINDOWS_STARTUP_VALUE_NAME) {
        Ok(reg_value) => {
            let current_exe_path: PathBuf = std::env::current_exe()?;

            let current_exe_path_str: &str = current_exe_path.to_str()
                .ok_or_else(|| err::NonUtf8ExecutablePathError)?;

            // make sure the path of the app which runs at startup is actually the path for this app
            Ok(current_exe_path_str == reg_value.as_str())
        },
        Err(error) => {
            match error.kind() {
                ErrorKind::NotFound => {
                    trace!("Found no windows startup registry value for {THIS_APPLICATION_NAME}");
                    Ok(false)
                },
                _ =>  Err(
                    err::RegistryOpsError::FailedToGetValueData {
                        key: String::from(WINDOWS_STARTUP_REGKEY_SUBPATH),
                        value: String::from(WINDOWS_STARTUP_VALUE_NAME),
                        source: error}.into()
                )
            }
        }
    };
}

// Checks the registry for whether Windows Explorer will hide file extensions.
#[instrument]
pub(crate) fn are_file_extensions_hidden() -> Result<bool> {
    let hive: RegKey = RegKey::predef(HKEY_CURRENT_USER);
    let win_explorer_advanced_key: RegKey = hive.open_subkey(WINDOWS_EXPLORER_REGKEY_SUBPATH)?;

    let value_data: u32 = win_explorer_advanced_key.get_value(HIDE_FILE_EXT_VALUE_NAME)?;
    return Ok(value_data != 0)
}

// Looks up a process by its name
#[instrument]
pub(crate) fn find_process_id_by_name(target_process_name: &str) -> Result<u32> {
    let all_processes_snapshot: HANDLE = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    // use during iteration
    let mut entry = PROCESSENTRY32 {
        dwSize: std::mem::size_of::<PROCESSENTRY32>() as u32,
        cntUsage: 0,
        th32ProcessID: 0,
        th32DefaultHeapID: 0,
        th32ModuleID: 0,
        cntThreads: 0,
        th32ParentProcessID: 0,
        pcPriClassBase: 0,
        dwFlags: 0,
        szExeFile: [0; 260],
    };

    let mut was_data_copied_to_entry: BOOL = unsafe { Process32First(all_processes_snapshot, &mut entry) };
    while was_data_copied_to_entry != 0 {
        let process_name: Cow<str> = String::from_utf8_lossy(&entry.szExeFile);

        let process_name: &str = process_name.trim_end_matches('\0');
        trace!("Evaluating process with name: {}", process_name);
        if process_name == target_process_name {
            return Ok(entry.th32ProcessID);
        }

        // reset the CHAR array to prevent leftovers influencing the next iteration
        entry.szExeFile.fill(0);
        was_data_copied_to_entry = unsafe { Process32Next(all_processes_snapshot, &mut entry) };
    }

    let target_process_name: String = String::from(target_process_name);
    Err(err::ProcessNotFoundError(target_process_name).into())
}

// Restart the Windows Explorer process. Any open windows will be lost during the restart.
fn restart_windows_explorer() -> Result<()> {
    let win_explorer_process_id: u32 = find_process_id_by_name("explorer.exe")?;
    trace!("Windows Explorer process id: {:?}", win_explorer_process_id);

    let win_explorer_process_handle: HANDLE = unsafe {
        OpenProcess(PROCESS_TERMINATE, BOOL::from(false), win_explorer_process_id)
    };
    trace!("Windows Explorer process id: {:?}", win_explorer_process_id);

    // The most simple and reliable way of restarting Windows Explorer is terminating its process
    // and letting Windows start another explorer process back up.
    // Alternatively, we can post a message to the Shell_TrayWnd window, as described here:
    // https://stackoverflow.com/questions/5689904/gracefully-exit-explorer-programmatically
    // but then we would be responsible for reliably waiting until explorer.exe is really dead
    // before starting it back up.
    match unsafe { TerminateProcess(win_explorer_process_handle, 0) } {
        0i32 => Err(err::UnableToRestartWindowsExplorer.into()),
        _ => Ok(())
    }
}

// Updates the registry so that Windows Explorer will not hide file extensions.
// This method returns whether a change was made.
// Note that it is possible for Windows Explorer to be out of sync with the registry.
#[instrument]
pub(crate) fn turn_off_file_extension_hiding() -> Result<bool> {
    let was_change_was_made: bool = set_or_update_registry_value(
        HKEY_CURRENT_USER,
        WINDOWS_EXPLORER_REGKEY_SUBPATH,
        HIDE_FILE_EXT_VALUE_NAME,
        0u32
    )?;

    // Windows Explorer won't pick up registry changes unless it is refreshed or restarted.
    // Refreshing Windows Explorer is difficult, so let's just restart it for now.
    if was_change_was_made {
        restart_windows_explorer()?;
    }
    Ok(was_change_was_made)
}

// Updates the registry so that the currently running program will run on Windows startup.
// This method returns whether a change was made.
// If the executable was moved, the registry value will be updated to reflect
// the executable's new location.
#[instrument]
pub(crate) fn run_this_program_at_startup() -> Result<bool> {
    let current_executable_path: PathBuf = std::env::current_exe()?;

    set_or_update_registry_value(
        HKEY_CURRENT_USER,
        WINDOWS_STARTUP_REGKEY_SUBPATH,
        WINDOWS_STARTUP_VALUE_NAME,
        current_executable_path.into_os_string()
    )
}

// Deletes the registry value for this program so that it will not run on Windows startup.
// This method returns whether a change was made.
#[instrument]
pub(crate) fn dont_run_this_program_at_startup() -> Result<bool> {
    if !will_app_run_at_startup()? {
        trace!("Executable already will not run at startup anyway");
        return Ok(false);
    }

    let hive: RegKey = RegKey::predef(HKEY_CURRENT_USER);
    let run_on_startup_key: RegKey = hive.open_subkey_with_flags(
        WINDOWS_STARTUP_REGKEY_SUBPATH, KEY_QUERY_VALUE | KEY_SET_VALUE
    )?;
    run_on_startup_key.delete_value(WINDOWS_STARTUP_VALUE_NAME)?;
    Ok(true)
}

// If a value with the given name already exists, update the value. Otherwise, create a new one.
// This method returns whether a change was made.
fn set_or_update_registry_value<V>(
    predefined_key: HKEY, subkey_path: &str, value_name: &str, desired_value: V
) -> Result<bool>
where
    V: ToRegValue + FromRegValue + Eq
{
    let transaction: Transaction = Transaction::new()?;

    let hive: RegKey = RegKey::predef(predefined_key);
    let subkey: RegKey = hive.open_subkey_transacted_with_flags(
        subkey_path, &transaction, KEY_QUERY_VALUE | KEY_SET_VALUE
    )?;

    return match subkey.get_value::<V, &str>(value_name) {
        Ok(current_value) => {
            // only change the value if it needs changing
            if current_value != desired_value {
                trace!("Existing value found which did not match the desired value.");
                subkey.set_value(value_name, &desired_value)?;
                transaction.commit()?;
                Ok(true)
            } else {
                trace!("Existing value found which matched the desired value.");
                transaction.commit()?;
                Ok(false)
            }
        },
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                trace!("No existing value found. Create the new value.");
                subkey.set_value(value_name, &desired_value)?;
                transaction.commit()?;
                Ok(true)
            },
            _ => {
                Err(Error::from(e))
            }
        }
    };
}

// Block until any value under the Windows Explorer Advanced registry key changes
pub(crate) fn wait_for_any_change_in_windows_explorer_regkey() -> Result<()> {
    let outer_key: RegKey = RegKey::predef(HKEY_CURRENT_USER);
    let subkey: RegKey = outer_key.open_subkey(WINDOWS_EXPLORER_REGKEY_SUBPATH)?;

    subkey.wait_for_key_or_value_change(false, REG_NOTIFY_CHANGE_LAST_SET, u32::MAX)?;
    Ok(())
}
