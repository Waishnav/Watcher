// KWin 6 script, manually installed and enabled by the user.
// Sends app class only, never the active window's title or geometry.
function publishActive(window) {
    var id = window && window.resourceClass ? String(window.resourceClass) : '';
    callDBus('org.watcher.FocusSink', '/org/watcher/FocusSink',
             'org.watcher.FocusSink', 'SetActiveApp', id);
}
workspace.windowActivated.connect(publishActive);
publishActive(workspace.activeWindow);
