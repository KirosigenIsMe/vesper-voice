//! Windows recording defaults only. We never disable the physical microphone:
//! Vesper still needs that endpoint to capture the original signal.
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use windows::{
    core::{w, IUnknown, IUnknown_Vtbl, Interface, GUID, HRESULT, PCWSTR},
    Win32::{
        Devices::Properties::DEVPKEY_Device_FriendlyName,
        Foundation::{
            CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0,
        },
        Media::Audio::{
            eCapture, eRender, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator,
            MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
        },
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
                COINIT_MULTITHREADED, STGM_READ,
            },
            Threading::{
                CreateMutexW, OpenProcess, ReleaseMutex, WaitForSingleObject, PROCESS_SYNCHRONIZE,
            },
        },
    },
};

type Result<T> = std::result::Result<T, String>;
fn win<T>(r: windows::core::Result<T>) -> Result<T> {
    r.map_err(|e| format!("Windows Audio: {e}"))
}
struct Com(bool);
impl Com {
    fn new() -> Result<Self> {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if hr.is_ok() {
            Ok(Self(true))
        } else if hr.0 == 0x80010106_u32 as i32 {
            Ok(Self(false))
        } else {
            Err(format!("COM: {hr:?}"))
        }
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

// IPolicyConfig (Windows 7+) layout is also used by AudioSwitcher/SoundSwitch.
// The ten methods between IUnknown and SetDefaultEndpoint are never invoked.
#[repr(C)]
struct PolicyVtable {
    base: IUnknown_Vtbl,
    unused: [usize; 10],
    set_default: unsafe extern "system" fn(*mut std::ffi::c_void, PCWSTR, ERole) -> HRESULT,
}
#[repr(transparent)]
#[derive(Clone)]
struct Policy(IUnknown);
unsafe impl Interface for Policy {
    type Vtable = PolicyVtable;
    const IID: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);
}
const POLICY_CLASS: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

#[derive(Debug, Clone, Serialize)]
pub struct Endpoint {
    pub id: String,
    pub name: String,
}
pub struct Cable {
    pub playback: Endpoint,
    pub capture: Endpoint,
}
fn enumerator() -> Result<IMMDeviceEnumerator> {
    win(unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) })
}
fn endpoint(device: IMMDevice) -> Result<Endpoint> {
    unsafe {
        let id = win(device.GetId())?;
        let text = id.to_string().map_err(|e| e.to_string());
        CoTaskMemFree(Some(id.0 as _));
        let store = win(device.OpenPropertyStore(STGM_READ))?;
        let name =
            win(store.GetValue(&DEVPKEY_Device_FriendlyName as *const _ as *const _))?.to_string();
        Ok(Endpoint { id: text?, name })
    }
}
fn endpoints(flow: EDataFlow) -> Result<Vec<Endpoint>> {
    let _com = Com::new()?;
    let e = enumerator()?;
    let devices = win(unsafe { e.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE) })?;
    let mut result = Vec::new();
    for i in 0..win(unsafe { devices.GetCount() })? {
        result.push(endpoint(win(unsafe { devices.Item(i) })?)?);
    }
    Ok(result)
}
fn is_named(name: &str, prefix: &str) -> bool {
    let lower = name.to_lowercase();
    lower == prefix || lower.starts_with(&format!("{prefix} ("))
}
pub fn cable() -> Result<Cable> {
    let playback = endpoints(eRender)?
        .into_iter()
        .find(|e| is_named(&e.name, "cable input"))
        .ok_or("Не найден CABLE Input. Установи VB-CABLE, затем обнови устройства.")?;
    let capture=endpoints(eCapture)?.into_iter().find(|e|is_named(&e.name,"cable output")).ok_or("Не найден CABLE Output. Заверши установку VB-CABLE; может понадобиться перезагрузка Windows.")?;
    Ok(Cable { playback, capture })
}
pub fn physical_inputs() -> Result<Vec<Endpoint>> {
    Ok(endpoints(eCapture)?
        .into_iter()
        .filter(|e| {
            let n = e.name.to_lowercase();
            !n.contains("cable") && !n.contains("virtual") && !n.contains("voicemeeter")
        })
        .collect())
}

trait Defaults {
    fn current(&self, role: usize) -> Result<Option<String>>;
    fn set(&self, role: usize, id: &str) -> Result<()>;
}
struct WindowsDefaults {
    enumerator: IMMDeviceEnumerator,
    policy: Policy,
    _com: Com,
}
impl WindowsDefaults {
    fn new() -> Result<Self> {
        let com = Com::new()?;
        Ok(Self {
            enumerator: enumerator()?,
            policy: win(unsafe { CoCreateInstance(&POLICY_CLASS, None, CLSCTX_ALL) })?,
            _com: com,
        })
    }
}
impl Defaults for WindowsDefaults {
    fn current(&self, role: usize) -> Result<Option<String>> {
        match unsafe {
            self.enumerator
                .GetDefaultAudioEndpoint(eCapture, ERole(role as i32))
        } {
            Ok(device) => Ok(Some(endpoint(device)?.id)),
            Err(e) if e.code().0 == 0x80070490_u32 as i32 => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set(&self, role: usize, id: &str) -> Result<()> {
        let text: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
        win(unsafe {
            (self.policy.vtable().set_default)(
                self.policy.as_raw(),
                PCWSTR(text.as_ptr()),
                ERole(role as i32),
            )
            .ok()
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Snapshot {
    token: String,
    pid: u32,
    target: String,
    previous: [Option<String>; 3],
}
fn restore<B: Defaults>(b: &B, s: &Snapshot) -> Result<()> {
    let mut errors = vec![];
    for role in 0..3 {
        if let Some(old) = &s.previous[role] {
            match b.current(role) {
                Ok(Some(now)) if now == s.target && old != &s.target => {
                    if let Err(e) = b.set(role, old) {
                        errors.push(e);
                    }
                }
                Ok(_) => {}
                Err(e) => errors.push(e),
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
fn apply<B: Defaults>(b: &B, s: &Snapshot) -> Result<()> {
    for role in 0..3 {
        if b.current(role)?.as_deref() != Some(&s.target) {
            if let Err(e) = b.set(role, &s.target) {
                let rollback = restore(b, s);
                return Err(format!(
                    "Не удалось назначить микрофон Vesper: {e}. Откат: {}",
                    rollback.err().unwrap_or_else(|| "выполнен".into())
                ));
            }
        }
    }
    for role in 0..3 {
        if b.current(role)?.as_deref() != Some(&s.target) {
            return Err("Windows не подтвердила переключение микрофона".into());
        }
    }
    Ok(())
}

pub struct Instance(HANDLE);
impl Instance {
    pub fn acquire() -> Result<Self> {
        let h = win(unsafe { CreateMutexW(None, false, w!("Local\\VesperVoiceStudio-1.1")) })?;
        let exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if exists {
            unsafe {
                let _ = CloseHandle(h);
            }
            return Err("Vesper уже запущен. Открой существующее окно.".into());
        }
        Ok(Self(h))
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
struct JournalLock(HANDLE);
impl JournalLock {
    fn new() -> Result<Self> {
        let h = win(unsafe { CreateMutexW(None, false, w!("Local\\VesperSystemRouteJournal")) })?;
        let result = unsafe { WaitForSingleObject(h, 3000) };
        if result != WAIT_OBJECT_0 && result != WAIT_ABANDONED {
            unsafe {
                let _ = CloseHandle(h);
            }
            return Err("Журнал маршрутизации занят".into());
        }
        Ok(Self(h))
    }
}
impl Drop for JournalLock {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}
fn journal(data: &Path) -> PathBuf {
    data.join("system-route.json")
}
fn read(path: &Path) -> Result<Option<Snapshot>> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| format!("Журнал маршрутизации: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn write(path: &Path, s: &Snapshot) -> Result<()> {
    let temp = path.with_extension("tmp");
    std::fs::write(
        &temp,
        serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temp, path).map_err(|e| e.to_string())
}
fn finish(path: &Path, s: &Snapshot) -> Result<()> {
    let _lock = JournalLock::new()?;
    let Some(current) = read(path)? else {
        return Ok(());
    };
    if current.token != s.token {
        return Ok(());
    }
    let b = WindowsDefaults::new()?;
    restore(&b, s)?;
    std::fs::remove_file(path).map_err(|e| e.to_string())
}
fn parent_alive(pid: u32) -> bool {
    unsafe {
        match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(h) => {
                let result = WaitForSingleObject(h, 0);
                let _ = CloseHandle(h);
                result != WAIT_OBJECT_0
            }
            Err(_) => false,
        }
    }
}
pub fn recover(data: &Path) -> Result<bool> {
    let path = journal(data);
    let old = {
        let _lock = JournalLock::new()?;
        read(&path)?
    };
    if let Some(s) = old {
        if s.pid != std::process::id() && parent_alive(s.pid) {
            return Err("Системным микрофоном управляет другой запущенный Vesper".into());
        }
        finish(&path, &s)?;
        return Ok(true);
    }
    Ok(false)
}

pub struct Guard {
    state: Snapshot,
    path: PathBuf,
    active: bool,
}
impl Guard {
    pub fn start(data: &Path, target: &str, physical_input: &str) -> Result<Self> {
        let _lock = JournalLock::new()?;
        let path = journal(data);
        if read(&path)?.is_some() {
            return Err("Сначала восстанови предыдущую маршрутизацию Windows".into());
        }
        let b = WindowsDefaults::new()?;
        let mut previous = [b.current(0)?, b.current(1)?, b.current(2)?];
        let fallback = physical_inputs()?
            .into_iter()
            .find(|e| e.name == physical_input)
            .map(|e| e.id);
        for old in &mut previous {
            if old.as_deref() == Some(target) {
                if let Some(id) = &fallback {
                    *old = Some(id.clone());
                }
            }
        }
        let pid = std::process::id();
        let token = format!(
            "{}-{}",
            pid,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let state = Snapshot {
            token: token.clone(),
            pid,
            target: target.into(),
            previous,
        };
        write(&path, &state)?;
        let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command
            .arg("--route-watchdog")
            .arg(&path)
            .arg(&token)
            .arg(pid.to_string());
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
        if let Err(e) = command.spawn() {
            let _ = std::fs::remove_file(&path);
            return Err(format!(
                "Не удалось запустить восстановление микрофона: {e}"
            ));
        }
        if let Err(e) = apply(&b, &state) {
            let rollback = restore(&b, &state);
            if rollback.is_ok() {
                let _ = std::fs::remove_file(&path);
            }
            return Err(e);
        }
        Ok(Self {
            state,
            path,
            active: true,
        })
    }
    pub fn ensure(&self) -> Result<()> {
        let _lock = JournalLock::new()?;
        let b = WindowsDefaults::new()?;
        apply(&b, &self.state)
    }
    pub fn stop(&mut self) -> Result<()> {
        if self.active {
            finish(&self.path, &self.state)?;
            self.active = false;
        }
        Ok(())
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
pub fn watchdog(path: &Path, token: &str, pid: u32) {
    unsafe {
        if let Ok(process) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            loop {
                if WaitForSingleObject(process, 500) == WAIT_OBJECT_0 {
                    break;
                }
                if !read(path)
                    .ok()
                    .flatten()
                    .is_some_and(|s| s.token == token && s.pid == pid)
                {
                    let _ = CloseHandle(process);
                    return;
                }
            }
            let _ = CloseHandle(process);
        }
    }
    if let Ok(Some(s)) = read(path) {
        if s.token == token && s.pid == pid {
            for _ in 0..3 {
                if finish(path, &s).is_ok() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }
    }
}
pub fn diagnose(path: &Path) -> Result<()> {
    let b = WindowsDefaults::new()?;
    let before = [b.current(0)?, b.current(1)?, b.current(2)?];
    // Verify the COM ABI by assigning each current role to itself. This does not
    // select a different device or require the virtual driver to be installed.
    for (role, id) in before.iter().enumerate() {
        if let Some(id) = id {
            b.set(role, id)?;
        }
    }
    let after = [b.current(0)?, b.current(1)?, b.current(2)?];
    if before != after {
        return Err("Диагностика изменила микрофон по умолчанию".into());
    }
    let inputs = endpoints(eCapture)?;
    let outputs = endpoints(eRender)?;
    let mut lifecycle = "SKIP: recording roles differ";
    if let Some(input) = inputs
        .iter()
        .find(|e| before.iter().all(|id| id.as_deref() == Some(e.id.as_str())))
    {
        let temp = std::env::temp_dir().join(format!("Vesper-route-check-{}", std::process::id()));
        std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
        let mut guard = Guard::start(&temp, &input.id, &input.name)?;
        if !journal(&temp).exists() {
            return Err("Журнал переключения не создан".into());
        }
        guard.ensure()?;
        guard.stop()?;
        if journal(&temp).exists() {
            return Err("Журнал не удалён после восстановления".into());
        }
        let stale = Snapshot {
            token: "diagnostic-stale".into(),
            pid: u32::MAX,
            target: input.id.clone(),
            previous: before.clone(),
        };
        write(&journal(&temp), &stale)?;
        if !recover(&temp)? || journal(&temp).exists() {
            return Err("Восстановление незавершённого переключения не выполнено".into());
        }
        let _ = std::fs::remove_dir(&temp);
        lifecycle = "PASS: guard start, watchdog spawn, ensure, stop, stale journal recovery";
    }
    let final_roles = [b.current(0)?, b.current(1)?, b.current(2)?];
    if final_roles != before {
        return Err("Роли микрофона не восстановлены после диагностики".into());
    }
    let report = serde_json::json!({"result":"PASS","lifecycle":lifecycle,"default_capture_roles":before,"inputs":inputs,"outputs":outputs,"vb_cable":cable().is_ok()});
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    struct Mock {
        roles: RefCell<[Option<String>; 3]>,
        fail: Cell<Option<usize>>,
    }
    impl Defaults for Mock {
        fn current(&self, r: usize) -> Result<Option<String>> {
            Ok(self.roles.borrow()[r].clone())
        }
        fn set(&self, r: usize, id: &str) -> Result<()> {
            if self.fail.get() == Some(r) {
                self.fail.set(None);
                return Err("injected failure".into());
            }
            self.roles.borrow_mut()[r] = Some(id.into());
            Ok(())
        }
    }
    fn setup() -> (Mock, Snapshot) {
        let previous = [
            Some("mic-a".into()),
            Some("mic-b".into()),
            Some("mic-c".into()),
        ];
        (
            Mock {
                roles: RefCell::new(previous.clone()),
                fail: Cell::new(None),
            },
            Snapshot {
                token: "test".into(),
                pid: 0,
                target: "cable".into(),
                previous,
            },
        )
    }
    #[test]
    fn switches_and_restores_all_roles() {
        let (b, s) = setup();
        apply(&b, &s).unwrap();
        assert!(b
            .roles
            .borrow()
            .iter()
            .all(|r| r.as_deref() == Some("cable")));
        restore(&b, &s).unwrap();
        assert_eq!(*b.roles.borrow(), s.previous);
        restore(&b, &s).unwrap();
        assert_eq!(*b.roles.borrow(), s.previous);
    }
    #[test]
    fn rolls_back_partial_failure() {
        let (b, s) = setup();
        b.fail.set(Some(1));
        assert!(apply(&b, &s).is_err());
        assert_eq!(*b.roles.borrow(), s.previous);
    }
    #[test]
    fn restore_preserves_manual_selection() {
        let (b, s) = setup();
        apply(&b, &s).unwrap();
        b.set(2, "new-headset").unwrap();
        restore(&b, &s).unwrap();
        assert_eq!(b.roles.borrow()[0], s.previous[0]);
        assert_eq!(b.roles.borrow()[2].as_deref(), Some("new-headset"));
    }
}
