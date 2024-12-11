// sample front-end code for the updater
import { check } from '@tauri-apps/plugin-updater';
import { ask, message } from '@tauri-apps/plugin-dialog';
import {invoke} from "@tauri-apps/api/core";

async function checkForAppUpdates(onUserClick: false) {
  const update = await check();
  if (update === null) {
    await message('Failed to check for updates.\nPlease try again later.', {
      title: 'Error',
      kind: 'error',
      okLabel: 'OK'
    });
    return;
  } else if (update?.available) {
    const yes = await ask(`Update to ${update.version} is available!\n\nRelease notes: ${update.body}`, {
      title: 'Update Available',
      kind: 'info',
      okLabel: 'Update',
      cancelLabel: 'Cancel'
    });
    if (yes) {
      await update.downloadAndInstall();
      await message('Update downloaded and installed successfully!', {
        title: 'Update Installed',
        kind: 'info',
        okLabel: 'OK'
      });
      await invoke('relaunch')
    }
  } else if (onUserClick) {
    await message('You are on the latest version. Stay awesome!', {
      title: 'No Update Available',
      kind: 'info',
      okLabel: 'OK'
    });
  }
}

export default checkForAppUpdates;
