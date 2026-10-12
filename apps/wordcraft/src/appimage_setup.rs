//! The SteamOS AppImage's start menu: Install/Update, Remove, Exit.
//!
//! Started from the AppImage without `--run` and without files, the app shows this menu instead of
//! the editor. Install/Update copies the AppImage to `~/.local/share/litera/Litera.AppImage` and
//! adds the menu entry, icons and document types; the entry starts that copy with `--run`, which
//! opens the editor. Remove deletes all of it, whatever version is installed.

use std::path::{Path, PathBuf};
use std::process::Command;

const APP_ID: &str = "ru.nookbat.litera";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whether to show the menu: started as an AppImage with nothing to open.
pub fn wanted(run: bool, nothing_to_open: bool) -> bool {
    !run && nothing_to_open && std::env::var_os("APPIMAGE").is_some()
}

/// Where the installed copy and its desktop files go.
struct Layout {
    data: PathBuf,
}

impl Layout {
    fn new() -> Option<Layout> {
        let data = match std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
            Some(d) => PathBuf::from(d),
            None => PathBuf::from(std::env::var_os("HOME")?).join(".local/share"),
        };
        Some(Layout { data })
    }
    fn app_dir(&self) -> PathBuf {
        self.data.join("litera")
    }
    fn appimage(&self) -> PathBuf {
        self.app_dir().join("Litera.AppImage")
    }
    fn version_file(&self) -> PathBuf {
        self.app_dir().join("version")
    }
    fn desktop(&self) -> PathBuf {
        self.data.join("applications").join(format!("{APP_ID}.desktop"))
    }
    fn mime(&self) -> PathBuf {
        self.data.join("mime/packages").join(format!("{APP_ID}.xml"))
    }
    fn icons(&self) -> PathBuf {
        self.data.join("icons/hicolor")
    }
    fn installed_version(&self) -> Option<String> {
        if !self.appimage().is_file() {
            return None;
        }
        let v = std::fs::read_to_string(self.version_file()).unwrap_or_default();
        Some(if v.trim().is_empty() { "?".into() } else { v.trim().to_string() })
    }
}

/// The icon files under `hicolor` (`<size>/apps/<APP_ID>.<ext>`), relative to it.
fn icon_files(hicolor: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(sizes) = std::fs::read_dir(hicolor) else { return out };
    for size in sizes.flatten() {
        for ext in ["png", "svg"] {
            let rel = PathBuf::from(size.file_name()).join("apps").join(format!("{APP_ID}.{ext}"));
            if hicolor.join(&rel).is_file() {
                out.push(rel);
            }
        }
    }
    out
}

fn desktop_entry(appimage: &Path) -> String {
    let exec = appimage.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"").replace('`', "\\`").replace('$', "\\$");
    let template = include_str!("../../../packaging/linux/ru.nookbat.litera.desktop");
    template
        .lines()
        .filter(|l| !l.starts_with("TryExec="))
        .map(|l| if l.starts_with("Exec=") { format!("Exec=\"{exec}\" --run %F") } else { l.to_string() })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

fn refresh_caches(layout: &Layout) {
    let runs: [(&str, Vec<PathBuf>); 4] = [
        ("update-desktop-database", vec![layout.data.join("applications")]),
        ("update-mime-database", vec![layout.data.join("mime")]),
        ("gtk-update-icon-cache", vec![PathBuf::from("-q"), PathBuf::from("-t"), PathBuf::from("-f"), layout.icons()]),
        ("kbuildsycoca6", vec![]),
    ];
    for (tool, args) in runs {
        if let Err(e) = Command::new(tool).args(&args).output() {
            log::info!("{tool}: {e}");
        }
    }
    if Command::new("kbuildsycoca6").arg("--version").output().is_err()
        && let Err(e) = Command::new("kbuildsycoca5").output()
    {
        log::info!("kbuildsycoca5: {e}");
    }
}

/// Copies the AppImage `source` (mounted at `appdir`) into place and registers it.
fn install(layout: &Layout, source: &Path, appdir: Option<&Path>) -> Result<(), String> {
    let target = layout.appimage();
    std::fs::create_dir_all(layout.app_dir()).map_err(|e| format!("{}: {e}", layout.app_dir().display()))?;
    let same = match (source.canonicalize(), target.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if !same {
        // Copied next to the target and renamed over it, so a running copy keeps working.
        let tmp = target.with_extension("part");
        std::fs::copy(source, &tmp).map_err(|e| format!("{}: {e}", source.display()))?;
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("{}: {e}", tmp.display()))?;
        }
        std::fs::rename(&tmp, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    }
    write_atomic(&layout.version_file(), VERSION.as_bytes()).map_err(|e| e.to_string())?;
    write_atomic(&layout.desktop(), desktop_entry(&target).as_bytes()).map_err(|e| format!("{}: {e}", layout.desktop().display()))?;
    write_atomic(&layout.mime(), include_bytes!("../../../packaging/linux/ru.nookbat.litera.mime.xml"))
        .map_err(|e| format!("{}: {e}", layout.mime().display()))?;
    if let Some(appdir) = appdir {
        let hicolor = appdir.join("usr/share/icons/hicolor");
        for rel in icon_files(&hicolor) {
            let bytes = std::fs::read(hicolor.join(&rel)).map_err(|e| format!("{}: {e}", rel.display()))?;
            write_atomic(&layout.icons().join(&rel), &bytes).map_err(|e| format!("{}: {e}", rel.display()))?;
        }
    }
    refresh_caches(layout);
    Ok(())
}

fn remove_file(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {e}", path.display())),
        _ => Ok(()),
    }
}

/// Removes every installed file, whatever version put it there.
fn uninstall(layout: &Layout) -> Result<(), String> {
    remove_file(&layout.desktop())?;
    remove_file(&layout.mime())?;
    for rel in icon_files(&layout.icons()) {
        remove_file(&layout.icons().join(rel))?;
    }
    match std::fs::remove_dir_all(layout.app_dir()) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(format!("{}: {e}", layout.app_dir().display())),
        _ => {}
    }
    refresh_caches(layout);
    Ok(())
}

struct Menu {
    layout: Option<Layout>,
    status: String,
}

impl eframe::App for Menu {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(format!("Литера {VERSION}"));
                let installed = self.layout.as_ref().and_then(Layout::installed_version);
                ui.label(match &installed {
                    Some(v) => format!("Установлена версия: {v}"),
                    None => "Программа не установлена".to_string(),
                });
                ui.add_space(12.0);
                let size = egui::vec2(260.0, 36.0);
                let install_label = if installed.is_some() { "Обновить" } else { "Установить" };
                if ui.add_sized(size, egui::Button::new(format!("1. {install_label}"))).clicked() {
                    let source = std::env::var_os("APPIMAGE").map(PathBuf::from);
                    let appdir = std::env::var_os("APPDIR").map(PathBuf::from);
                    self.status = match (self.layout.as_ref(), source) {
                        (Some(layout), Some(source)) => match install(layout, &source, appdir.as_deref()) {
                            Ok(()) => "Готово: Литера установлена. Ярлык — в меню приложений.".into(),
                            Err(e) => format!("Ошибка установки: {e}"),
                        },
                        _ => "Ошибка: не найден домашний каталог или файл AppImage".into(),
                    };
                }
                ui.add_space(4.0);
                if ui.add_sized(size, egui::Button::new("2. Удалить")).clicked() {
                    self.status = match self.layout.as_ref().map(uninstall) {
                        Some(Ok(())) => "Готово: Литера удалена.".into(),
                        Some(Err(e)) => format!("Ошибка удаления: {e}"),
                        None => "Ошибка: не найден домашний каталог".into(),
                    };
                }
                ui.add_space(4.0);
                if ui.add_sized(size, egui::Button::new("3. Выход")).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                ui.add_space(10.0);
                ui.label(&self.status);
            });
        });
    }
}

pub fn run(icon: Option<egui::IconData>) -> eframe::Result {
    let mut viewport =
        egui::ViewportBuilder::default().with_title("Литера — установка").with_app_id(APP_ID).with_inner_size([440.0, 300.0]).with_resizable(false);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Литера — установка", options, Box::new(|_| Ok(Box::new(Menu { layout: Layout::new(), status: String::new() }))))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn install_then_remove_leaves_nothing_behind() {
        let dir = std::env::temp_dir().join(format!("litera-setup-{}", std::process::id()));
        let layout = Layout { data: dir.join("share") };
        let fake = dir.join("Litera-test.AppImage");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&fake, b"appimage").unwrap();
        let appdir = dir.join("AppDir/usr/share/icons/hicolor/256x256/apps");
        std::fs::create_dir_all(&appdir).unwrap();
        std::fs::write(appdir.join(format!("{APP_ID}.png")), b"png").unwrap();
        install(&layout, &fake, Some(&dir.join("AppDir"))).unwrap();
        assert_eq!(std::fs::read(layout.appimage()).unwrap(), b"appimage");
        assert_eq!(layout.installed_version().as_deref(), Some(VERSION));
        let entry = std::fs::read_to_string(layout.desktop()).unwrap();
        assert!(entry.contains(&format!("Exec=\"{}\" --run %F", layout.appimage().display())), "{entry}");
        assert!(layout.icons().join(format!("256x256/apps/{APP_ID}.png")).is_file());
        // A newer installed version is removed too.
        std::fs::write(layout.version_file(), "9.9.9").unwrap();
        uninstall(&layout).unwrap();
        assert!(!layout.app_dir().exists() && !layout.desktop().exists() && !layout.mime().exists());
        assert!(!layout.icons().join(format!("256x256/apps/{APP_ID}.png")).exists());
        assert_eq!(layout.installed_version(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
