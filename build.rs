// Icône et nom de l'application dans l'exe : visibles dans l'Bilan Windows,
// la barre des tâches et le Gestionnaire des tâches (qui affiche « Bilan »).
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico")
            .set("FileDescription", "Bilan")
            .set("ProductName", "Bilan")
            .set("InternalName", "bilan")
            .set("OriginalFilename", "Bilan.exe")
            .set("CompanyName", "Bilan")
            .set("LegalCopyright", "");
        if let Err(e) = res.compile() {
            println!("cargo:warning=icône non intégrée : {e}");
        }
    }
}
