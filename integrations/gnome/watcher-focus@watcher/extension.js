// GNOME Shell 45+ extension; install and enable explicitly.
// Sends only application IDs over the local *user session* bus.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class WatcherFocusBridge extends Extension {
    enable() {
        this._focusId = global.display.connect('notify::focus-window', () => this._sendFocus());
        this._timer = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 5, () => {
            // Also a heartbeat. If daemon restarted, it gets the current app.
            this._sendFocus();
            return GLib.SOURCE_CONTINUE;
        });
        this._sendFocus();
    }

    _sendFocus() {
        const win = global.display.focus_window;
        // GNOME Shell can map a Meta.Window to a desktop entry; app_id is more
        // stable than titles or executable paths. Fall back to wm_class.
        const appId = win?.get_gtk_application_id?.() || win?.get_wm_class?.() || '';
        Gio.DBus.session.call(
            'org.watcher.FocusSink', '/org/watcher/FocusSink',
            'org.watcher.FocusSink', 'SetActiveApp',
            new GLib.Variant('(s)', [appId]), new GLib.VariantType('(b)'),
            Gio.DBusCallFlags.NONE, 1000, null,
            (conn, result) => {
                try { conn.call_finish(result); } catch (_) { /* daemon absent */ }
            },
        );
    }

    disable() {
        if (this._focusId) {
            global.display.disconnect(this._focusId);
            this._focusId = 0;
        }
        if (this._timer) {
            GLib.Source.remove(this._timer);
            this._timer = 0;
        }
    }
}
