fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set("ProductName", "Synector");
        res.set("FileDescription", "Synector");
        res.set("CompanyName", "psyattack");
        res.set("LegalCopyright", "psyattack");
        res.set("OriginalFilename", "Synector.exe");
        res.set("InternalName", "Synector");
        res.set("FileVersion", "2.6.0.0");
        res.set("ProductVersion", "2.6.0.0");
        res.set_version_info(winres::VersionInfo::FILEVERSION, 0x0002000600000000);
        res.set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0002000600000000);
        let _ = res.compile();
    }
}
