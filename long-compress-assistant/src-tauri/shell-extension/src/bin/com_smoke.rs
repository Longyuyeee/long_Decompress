use windows::core::{GUID, PWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{IExplorerCommand, ECF_HASSUBCOMMANDS};

const EXPLORER_COMMAND_GUID: GUID =
    GUID::from_u128(0xd4bba0b2_6a58_4d40_8b79_ba50c54e8d4a);

struct ComApartment;

impl Drop for ComApartment {
    fn drop(&mut self) {
        // SAFETY: this guard is created only after CoInitializeEx succeeds on
        // the current thread and is dropped on that same thread.
        unsafe { CoUninitialize() };
    }
}

fn take_string(value: PWSTR) -> windows::core::Result<String> {
    // SAFETY: IExplorerCommand::GetTitle returns a null-terminated string
    // allocated with CoTaskMemAlloc. It remains valid until freed below.
    let result = unsafe { value.to_string() };
    // SAFETY: the pointer was allocated by the COM task allocator.
    unsafe { CoTaskMemFree(Some(value.0.cast())) };
    Ok(result?)
}

fn main() -> windows::core::Result<()> {
    // SAFETY: no COM calls have been made on this short-lived worker thread.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
    let _apartment = ComApartment;

    // SAFETY: the registered class is created in-process and no outer unknown
    // is supplied, matching the shell extension's class factory contract.
    let root: IExplorerCommand = unsafe {
        CoCreateInstance(&EXPLORER_COMMAND_GUID, None, CLSCTX_INPROC_SERVER)?
    };
    // SAFETY: the COM interface is valid for the lifetime of `root`.
    let flags = unsafe { root.GetFlags()? };
    assert_ne!(flags & ECF_HASSUBCOMMANDS.0 as u32, 0);

    // SAFETY: the COM interface is valid for the duration of enumeration.
    let commands = unsafe { root.EnumSubCommands()? };
    let mut titles = Vec::new();
    loop {
        let mut slot = [None];
        let mut fetched = 0;
        // SAFETY: `slot` has capacity for one command and `fetched` is a valid
        // writable count pointer for the duration of the call.
        unsafe { commands.Next(&mut slot, Some(&mut fetched)).ok()? };
        if fetched == 0 {
            break;
        }
        let command = slot[0].take().expect("enumerator returned no command");
        // SAFETY: passing None asks for a selection-independent display name.
        let title = unsafe { command.GetTitle(None)? };
        titles.push(take_string(title)?);
    }

    let expected = [
        "一键解压（推荐）",
        "解压到此处",
        "解压到同名文件夹",
        "测试压缩包完整性",
        "压缩为 ZIP",
        "压缩为 7Z",
        "更多压缩选项…",
    ];
    assert_eq!(titles, expected);
    println!("COM activation succeeded; commands: {}", titles.join(" | "));
    Ok(())
}
