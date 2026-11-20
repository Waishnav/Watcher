import Gio from 'gi://Gio';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const BUS_NAME = 'org.micro_watcher.Focus';
const OBJECT_PATH = '/org/micro_watcher/Focus';
const VERSION = '1';

const INTERFACE_XML = `
<node>
  <interface name="org.micro_watcher.Focus">
    <method name="GetActiveWindow">
      <arg type="s" direction="out" name="app_id"/>
      <arg type="s" direction="out" name="wm_class"/>
      <arg type="s" direction="out" name="title"/>
      <arg type="s" direction="out" name="pid"/>
    </method>
    <method name="GetVersion">
      <arg type="s" direction="out" name="version"/>
    </method>
  </interface>
</node>
`;

export default class MicroWatcherFocusExtension extends Extension {
    enable() {
        if (this._dbus)
            return;

        this._dbus = Gio.DBusExportedObject.wrapJSObject(INTERFACE_XML, this);
        this._dbus.export(Gio.DBus.session, OBJECT_PATH);
        this._ownerId = Gio.bus_own_name_on_connection(
            Gio.DBus.session,
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            null,
            null);
    }

    disable() {
        if (this._dbus) {
            this._dbus.unexport();
            this._dbus = null;
        }

        if (this._ownerId) {
            Gio.bus_unown_name(this._ownerId);
            this._ownerId = 0;
        }
    }

    GetActiveWindow() {
        const window = global.display.focus_window;
        if (!window)
            return ['', '', '', ''];

        let appId = '';
        let wmClass = '';
        let title = '';
        let pid = '';

        try {
            appId = window.get_gtk_application_id() ?? '';
        } catch (e) {
            appId = '';
        }

        try {
            wmClass = window.get_wm_class() ?? '';
        } catch (e) {
            wmClass = '';
        }

        try {
            title = window.get_title() ?? '';
        } catch (e) {
            title = '';
        }

        try {
            pid = String(window.get_pid() ?? '');
        } catch (e) {
            pid = '';
        }

        return [appId, wmClass, title, pid];
    }

    GetVersion() {
        return VERSION;
    }
}
