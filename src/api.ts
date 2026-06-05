import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface ImageMeta { id: string; path: string; }
export interface GenResult { path: string; url: string; }
export interface AppSettings { gallery_dir: string|null; snapshot_dir: string|null; fullscreen: boolean; child_label: string; }

export const api = {
  getSettings: () => invoke<AppSettings>('get_settings'),
  setSettings: (s: AppSettings) => invoke<void>('set_settings', { new: s }),
  listGallery: () => invoke<ImageMeta[]>('list_gallery'),
  drawRandom: () => invoke<ImageMeta>('draw_random'),
  generateImage: (transcript: string) => invoke<GenResult>('generate_image', { transcript }),
  editImage: (prevUrl: string, instruction: string) => invoke<GenResult>('edit_image', { prevUrl, instruction }),
  asrStart: () => invoke<void>('asr_start'),
  asrStop: () => invoke<void>('asr_stop'),
  saveSnapshot: (pngBase64: string) => invoke<string>('save_snapshot', { pngBase64 }),
  checkConnectivity: () => invoke<boolean>('check_connectivity'),
};

export function onEvent<T>(name: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (e) => cb(e.payload));
}

export const fileUrl = (p: string) => convertFileSrc(p);
