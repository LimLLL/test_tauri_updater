// sample front-end code for the updater
import { check } from '@tauri-apps/plugin-updater';
import { ask, message } from '@tauri-apps/plugin-dialog';
import {invoke} from "@tauri-apps/api/core";
import type { Dispatch, SetStateAction } from 'react';

async function checkForAppUpdates(setDownloaded: Dispatch<SetStateAction<number>>, setContentLength: Dispatch<SetStateAction<number>>) {
  const update = await check();
  if (update === null) {
    await message('You are on the latest version. Stay awesome!', {
      title: 'Success',
      kind: 'info',
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
      await update.downloadAndInstall(async (event) => {
        switch (event.event) {
          case 'Started':
            setContentLength(event.data.contentLength ?? 0);
            break;
          case 'Progress':
            setDownloaded(prevState => {
              return prevState + (event.data.chunkLength ?? 0)
            });

            break;
          case 'Finished':
            break;
        }
        await invoke('relaunch');
      })
      // await message('Update downloaded and installed successfully!', {
      //   title: 'Update Installed',
      //   kind: 'info',
      //   okLabel: 'OK'
      // });
      // await invoke('relaunch')
    }
  }
}

export default checkForAppUpdates;
