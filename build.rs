fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set("ProductName", "ProcessHandler");
        res.set("FileDescription", "ProcessHandler");
        res.set("CompanyName", "psyattack");
        res.set("LegalCopyright", "psyattack");
        res.set("OriginalFilename", "ProcessHandler.exe");
        res.set("InternalName", "ProcessHandler");
        res.set("FileVersion", "2.5.0.0");
        res.set("ProductVersion", "2.5.0.0");
        res.set_version_info(winres::VersionInfo::FILEVERSION, 0x0002000500000000);
        res.set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0002000500000000);
        let _ = res.compile();
    }
}
