// Loaded into KWin by masterhubd: reports the focused window's class for per-app profiles.
// Plasma 6 names first, the Plasma 5 ones (`client…`) as a fallback.

// Plasma's own surfaces: the launcher, tray popups, KRunner and the like take focus briefly
// and should not switch profiles.
var IGNORED = ["plasmashell", "org.kde.plasmashell", "krunner", "org.kde.krunner"];

function send(method, arg) {
    callDBus("org.openmasterhub.Focus", "/org/openmasterhub/Focus", "org.openmasterhub.Focus1", method, arg);
}

// Errors go to masterhubd's log, where they are easier to find than in KWin's.
function guarded(f) {
    return function (w) {
        try {
            f(w);
        } catch (e) {
            send("ScriptError", String(e));
        }
    };
}

var report = guarded(function (w) {
    // Clicking the desktop focuses Plasma's desktop window (class plasmashell): no app, so the
    // default profile.
    if (!w || w.desktopWindow) {
        send("ActiveWindow", "");
        return;
    }
    if (w.specialWindow || w.dock || IGNORED.indexOf(String(w.resourceClass)) >= 0) {
        return;
    }
    send("ActiveWindow", String(w.resourceClass));
});

guarded(function () {
    (workspace.windowActivated || workspace.clientActivated).connect(report);
    report(workspace.activeWindow !== undefined ? workspace.activeWindow : workspace.activeClient);
})();
