import { defineConfig } from 'wxt';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  modules: ['@wxt-dev/module-react'],
  vite: () => ({ plugins: [tailwindcss()] }),
  manifest: ({ browser }) => ({
    name: 'Watcher — Browser & Device Time',
    description: 'Private, offline-first browser usage and optional local device analytics.',
    permissions: ['storage', 'tabs', 'idle', 'alarms', 'nativeMessaging'],
    action: { default_title: 'Open Watcher sidepanel' },
    ...(browser === 'firefox'
      ? { browser_specific_settings: { gecko: { id: 'watcher@watcher.local', data_collection_permissions: { required: ['websiteActivity'] } } } }
      : {}),
  }),
});
