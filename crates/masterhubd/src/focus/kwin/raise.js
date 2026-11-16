// One-shot script masterhubd loads into KWin to focus the window of a process, or of its
// nearest ancestor that has one (the terminal a CLI runs in). masterhubd fills in the values.
var PIDS = __PIDS__; // the process first, then its parents
var TOKEN = __TOKEN__;
var TITLE = __TITLE__; // lower case; "" when unknown

function pick() {
    var stack = workspace.stackingOrder; // bottom to top
    for (var i = 0; i < PIDS.length; i++) {
        var own = [];
        for (var j = stack.length - 1; j >= 0; j--) {
            var w = stack[j];
            if (w && w.pid === PIDS[i] && w.normalWindow) {
                own.push(w);
            }
        }
        if (own.length === 0) {
            continue;
        }
        if (TITLE) {
            for (var k = 0; k < own.length; k++) {
                if (String(own[k].caption).toLowerCase().indexOf(TITLE) >= 0) {
                    return own[k];
                }
            }
        }
        return own[0]; // the topmost
    }
    return null;
}

var target = pick();
if (target) {
    if (target.minimized) {
        target.minimized = false;
    }
    // Activating also switches to the window's virtual desktop.
    if (workspace.activeWindow !== undefined) {
        workspace.activeWindow = target;
    } else {
        workspace.activeClient = target;
    }
}
callDBus("org.openmasterhub.Focus", "/org/openmasterhub/Focus", "org.openmasterhub.Focus1", "Raised", TOKEN, target !== null);
