// Loaded into KWin by masterhubd: reports the focused window's class for per-app profiles.
// Plasma 6 names first, the Plasma 5 ones (`client…`) as a fallback.

// Plasma's own surfaces: the launcher, tray popups, KRunner and the like take focus briefly
// and should not switch profiles.
var IGNORED = ["plasmashell", "org.kde.plasmashell", "krunner", "org.kde.krunner"];

function report(w) {
    if (w && (w.specialWindow || w.dock || w.desktopWindow || IGNORED.indexOf(String(w.resourceClass)) >= 0)) {
        return;
    }
    callDBus(
        "org.openmasterhub.Focus",
        "/org/openmasterhub/Focus",
        "org.openmasterhub.Focus1",
        "ActiveWindow",
        w ? String(w.resourceClass) : ""
    );
}

(workspace.windowActivated || workspace.clientActivated).connect(report);
report(workspace.activeWindow !== undefined ? workspace.activeWindow : workspace.activeClient);
