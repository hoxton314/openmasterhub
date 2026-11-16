// One-shot script masterhubd loads into KWin to list the classes of the open windows, as
// suggestions for a profile's apps. masterhubd fills in the token.
var TOKEN = __TOKEN__;
// Plasma's own surfaces, as in focus.js.
var IGNORED = ["plasmashell", "org.kde.plasmashell", "krunner", "org.kde.krunner"];

var classes = [];
var stack = workspace.stackingOrder;
for (var i = 0; i < stack.length; i++) {
    var w = stack[i];
    var cls = w ? String(w.resourceClass) : "";
    if (w && w.normalWindow && cls && IGNORED.indexOf(cls) < 0) {
        classes.push(cls);
    }
}
// One string, so the D-Bus type does not depend on how KWin converts a JS array.
callDBus("org.openmasterhub.Focus", "/org/openmasterhub/Focus", "org.openmasterhub.Focus1", "Windows", TOKEN, classes.join("\n"));
