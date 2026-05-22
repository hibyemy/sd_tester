#[cfg(windows)]
pub fn is_running_as_admin() -> bool {
    use windows_sys::Win32::Security::{
        AllocateAndInitializeSid, CheckTokenMembership, FreeSid, SECURITY_NT_AUTHORITY,
        SID_IDENTIFIER_AUTHORITY,
    };
    const SECURITY_BUILTIN_DOMAIN_RID: u32 = 0x0000_0020;
    const DOMAIN_ALIAS_RID_ADMINS: u32 = 0x0000_0220;

    unsafe {
        let mut admin_group = std::ptr::null_mut();
        let nt_authority = SID_IDENTIFIER_AUTHORITY {
            Value: SECURITY_NT_AUTHORITY.Value,
        };
        let ok = AllocateAndInitializeSid(
            &nt_authority,
            2,
            SECURITY_BUILTIN_DOMAIN_RID,
            DOMAIN_ALIAS_RID_ADMINS,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut admin_group,
        );
        if ok == 0 {
            return false;
        }

        let mut is_member = 0;
        let result = CheckTokenMembership(std::ptr::null_mut(), admin_group, &mut is_member);
        FreeSid(admin_group);
        result != 0 && is_member != 0
    }
}

pub fn relaunch_as_admin() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let status = runas::Command::new(exe).gui(true).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("elevation relaunch failed"))
    }
}
