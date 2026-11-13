// Reports the focused window's class to masterhubd (OpenMasterHUB) for per-app profiles.
// Plasma 6 KWin scripting API.

function report(w) {
    // Ignore panels and the desktop so clicking them doesn't switch profiles.
    if (w && (w.dock || w.desktopWindow)) {
        return;
    }
    var cls = w ? String(w.resourceClass) : "";
    print("openmasterhub-focus: " + cls);
    callDBus(
        "org.openmasterhub.Focus",
        "/org/openmasterhub/Focus",
        "org.openmasterhub.Focus1",
        "ActiveWindow",
        cls
    );
}

workspace.windowActivated.connect(report);
report(workspace.activeWindow);
