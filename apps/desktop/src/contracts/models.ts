export type Locale = 'zh-CN' | 'en-US' | 'ja-JP';

export type BatchState =
  | 'empty'
  | 'queue'
  | 'compressing'
  | 'compressed'
  | 'uploading'
  | 'uploaded'
  | 'failed'
  | 'cancelling';

export type CompressStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled';
export type UploadStatus = 'none' | 'pending' | 'running' | 'done' | 'failed' | 'cancelled';

export type ErrorCode =
  | 'unsupported_format'
  | 'file_not_found'
  | 'encoder_missing'
  | 'compression_failed'
  | 'upload_disabled'
  | 'upload_failed'
  | 'credential_missing'
  | 'permission_denied'
  | 'network_timeout'
  | 'cancelled'
  | 'cancel_required'
  | 'batch_not_found'
  | 'asset_not_found'
  | 'asset_not_uploaded'
  | 'route_not_found'
  | 'route_delete_forbidden'
  | 'route_name_required'
  | 'route_public_base_url_required'
  | 'route_endpoint_required'
  | 'route_bucket_required'
  | 'route_repository_required'
  | 'route_local_folder_required'
  | 'github_repository_invalid'
  | 'github_enterprise_base_url_required'
  | 'route_provider_unsupported'
  | 'custom_api_method_invalid'
  | 'custom_api_body_invalid'
  | 'invalid_history_scope'
  | 'state_poisoned'
  | 'storage_error'
  | 'clipboard_failed'
  | 'database_unavailable'
  | 'unknown';

export type SafeError = {
  code: ErrorCode;
  message: string;
  recoverable: boolean;
  retryable: boolean;
};

export type FileInput = {
  path: string;
  name: string;
  size: number;
  mime?: string;
  previewUrl?: string;
  isDirectory?: boolean;
};

export type CopyFormat = 'url' | 'markdown' | 'html' | 'custom';

export type CompressionMode = 'balanced' | 'tiny' | 'lossless' | 'retina' | 'extreme_avif' | 'custom';
export type OutputFormat = 'original' | 'webp' | 'avif' | 'jpg' | 'png';

export type CompressionOptions = {
  mode: CompressionMode;
  quality: number;
  encoderDirectory?: string;
  maxWidth?: number;
  maxHeight?: number;
  outputFormat: OutputFormat;
  stripMetadata: boolean;
  preserveColorProfile: boolean;
  keepOriginal: boolean;
  fallbackWhenLarger: boolean;
};


export type CompressionToolStatus = {
  name: string;
  available: boolean;
  path?: string;
  purpose: string;
};

export type AssetState = {
  id: string;
  batchId: string;
  name: string;
  sourcePath: string;
  outputPath?: string;
  sourceSize: number;
  outputSize?: number;
  mime?: string;
  previewUrl?: string;
  width?: number;
  height?: number;
  savedRatio?: number;
  compressStatus: CompressStatus;
  uploadStatus: UploadStatus;
  compressProgress: number;
  uploadProgress: number;
  selected: boolean;
  routeId?: string;
  publicUrl?: string;
  error?: SafeError;
};

export type Batch = {
  id: string;
  status: BatchState;
  assets: AssetState[];
  createdAt: string;
  updatedAt: string;
};

export type RouteProvider =
  | 's3'
  | 'aliyun_oss'
  | 'tencent_cos'
  | 'qiniu'
  | 'custom_api'
  | 'local_folder'
  | 'github';

export type CustomApiMethod = 'POST' | 'PUT' | 'PATCH';
export type CustomApiBody = 'binary' | 'multipart';

export type Route = {
  id: string;
  name: string;
  provider: RouteProvider;
  credentialRef: string;
  endpoint?: string;
  bucket?: string;
  region?: string;
  pathTemplate: string;
  publicBaseUrl: string;
  cacheControl: string;
  defaultCopyFormat: CopyFormat;
  customLinkTemplate?: string;
  customApiMethod?: CustomApiMethod;
  customApiBody?: CustomApiBody;
  customApiFileField?: string;
  customApiHeaders?: string;
  customApiUrlPath?: string;
  concurrency: number;
  createdAt: string;
  updatedAt: string;
};

export type RouteCredentialInput = {
  accessKeyId?: string;
  secretAccessKey?: string;
  sessionToken?: string;
  apiToken?: string;
};

export type RouteDraft = {
  id?: string;
  name: string;
  provider: RouteProvider;
  endpoint?: string;
  bucket?: string;
  region?: string;
  pathTemplate: string;
  publicBaseUrl: string;
  cacheControl: string;
  defaultCopyFormat: CopyFormat;
  customLinkTemplate?: string;
  customApiMethod?: CustomApiMethod;
  customApiBody?: CustomApiBody;
  customApiFileField?: string;
  customApiHeaders?: string;
  customApiUrlPath?: string;
  concurrency: number;
  credential?: RouteCredentialInput;
};

export type RouteCredentialStatus = {
  routeId: string;
  credentialRef: string;
  configured: boolean;
  checkedAt: string;
};

export type RouteTestResult = {
  routeId: string;
  ok: boolean;
  messageCode?: 'route_test_success' | ErrorCode;
  message: string;
  checkedAt: string;
  publicBaseUrl?: string;
};

export type AppSettings = {
  locale: Locale;
  defaultRouteId?: string;
  uploadEnabled: boolean;
  copyFormat: CopyFormat;
  hideLocalPath: boolean;
  deleteLocalOutputsOnClear: boolean;
  encoderDirectory?: string;
  compressionMode: CompressionMode;
  compressionQuality: number;
  maxWidth?: number;
  maxHeight?: number;
  outputFormat: OutputFormat;
  stripMetadata: boolean;
  preserveColorProfile: boolean;
  keepOriginal: boolean;
  fallbackWhenLarger: boolean;
};

export type HistoryItem = {
  id: string;
  batchId: string;
  fileName: string;
  sourcePath?: string;
  outputPath?: string;
  sourceSize: number;
  outputSize?: number;
  savedRatio?: number;
  outputFormat?: string;
  width?: number;
  height?: number;
  routeName?: string;
  routeId?: string;
  publicUrl?: string;
  compressStatus: CompressStatus;
  uploadStatus: UploadStatus;
  canCopy: boolean;
  createdAt: string;
  uploadedAt?: string;
  copiedAt?: string;
};

export type HistoryQuery = {
  keyword?: string;
  routeId?: string;
  status?: 'compressed' | 'uploaded' | 'failed';
  page: number;
  pageSize: number;
};

export type HistoryPage = {
  items: HistoryItem[];
  page: number;
  pageSize: number;
  total: number;
};

export type ClearHistoryInput = {
  scope: 'all' | 'selected';
  ids?: string[];
  clearLocalOutputs: boolean;
};

export type CopyFallback = {
  publicUrl?: string;
  fileName?: string;
  alt?: string;
  width?: number;
  height?: number;
  customTemplate?: string;
};
