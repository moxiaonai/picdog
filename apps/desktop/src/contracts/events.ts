import type { Batch, BatchState, SafeError } from './models';

export const TASK_EVENT_NAME = 'picdog://task-event';

export type OutputAsset = {
  outputPath?: string;
  outputSize?: number;
  mime?: string;
  savedRatio?: number;
  width?: number;
  height?: number;
};

export type TaskEvent =
  | { type: 'batch.created'; batchId: string; batch: Batch }
  | { type: 'batch.status.changed'; batchId: string; status: BatchState }
  | { type: 'asset.compress.started'; batchId: string; assetId: string }
  | { type: 'asset.compress.progress'; batchId: string; assetId: string; progress: number }
  | { type: 'asset.compress.done'; batchId: string; assetId: string; output: OutputAsset }
  | { type: 'asset.upload.started'; batchId: string; assetId: string }
  | { type: 'asset.upload.progress'; batchId: string; assetId: string; progress: number }
  | { type: 'asset.upload.done'; batchId: string; assetId: string; url: string }
  | { type: 'asset.cancelled'; batchId: string; assetId: string; stage: 'compress' | 'upload' }
  | { type: 'asset.failed'; batchId: string; assetId: string; stage: 'compress' | 'upload'; error: SafeError }
  | { type: 'queue.idle'; batchId: string };
