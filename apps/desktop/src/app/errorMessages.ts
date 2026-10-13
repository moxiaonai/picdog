import type { ErrorCode, Locale, RouteTestResult, SafeError } from '../contracts/models';

type UiMessageKey =
  | 'operationCancelled'
  | 'operationFailed'
  | 'noUploadTarget'
  | 'partialUploadFailed'
  | 'routeTestSuccess';

const errorMessages: Record<Locale, Partial<Record<ErrorCode, string>>> = {
  'zh-CN': {
    unsupported_format: '暂不支持这种图片格式。',
    file_not_found: '找不到原始文件，请确认文件仍在本机。',
    encoder_missing: '缺少本机编码器，请在设置里安装或选择编码器目录。',
    compression_failed: '压缩失败，请检查图片文件或压缩设置。',
    upload_disabled: '上传功能已关闭，请先在 CDN 配置中开启。',
    upload_failed: '上传失败，请检查上传目标配置。',
    credential_missing: '缺少上传凭据，请先在 CDN 配置里保存凭据。',
    permission_denied: '没有权限完成该操作，请检查文件或目标权限。',
    network_timeout: '网络请求超时，请稍后重试。',
    cancelled: '操作已取消。',
    cancel_required: '当前任务需要先取消后再继续。',
    batch_not_found: '找不到当前批次，请重新选择图片。',
    asset_not_found: '找不到这张图片，请重新选择。',
    asset_not_uploaded: '上传完成后才能复制链接。',
    route_not_found: '找不到上传目标，请重新选择或新建目标。',
    route_delete_forbidden: '当前上传目标不能删除。',
    route_name_required: '上传目标名称不能为空。',
    route_public_base_url_required: '上传目标缺少 CDN 域名。',
    route_endpoint_required: '上传目标缺少 Endpoint。',
    route_bucket_required: '上传目标缺少 Bucket。',
    route_repository_required: '上传目标缺少 GitHub 仓库。',
    route_local_folder_required: '上传目标缺少本地目录。',
    github_repository_invalid: 'GitHub 仓库格式应为 owner/repo 或仓库 URL。',
    github_enterprise_base_url_required: 'GitHub Enterprise 目标需要填写 Raw / CDN 域名。',
    route_provider_unsupported: '暂不支持该上传目标类型。',
    custom_api_method_invalid: 'Custom API 请求方法只支持 POST、PUT 或 PATCH。',
    custom_api_body_invalid: 'Custom API 请求体只支持 binary 或 multipart。',
    invalid_history_scope: '清除历史的范围无效。',
    state_poisoned: '应用状态暂时不可用，请重试。',
    storage_error: '本地数据读写失败，请重试。',
    clipboard_failed: '写入剪贴板失败，请手动复制。',
    database_unavailable: '本地数据库不可用，请重启应用后重试。',
    unknown: '操作失败，请检查设置后重试。'
  },
  'en-US': {
    unsupported_format: 'This image format is not supported yet.',
    file_not_found: 'The original file could not be found. Make sure it still exists on this Mac.',
    encoder_missing: 'A local encoder is missing. Install it or choose an encoder folder in Settings.',
    compression_failed: 'Compression failed. Check the image file or compression settings.',
    upload_disabled: 'Upload is disabled. Enable it in CDN settings first.',
    upload_failed: 'Upload failed. Check the upload target settings.',
    credential_missing: 'Upload credentials are missing. Save them in CDN settings first.',
    permission_denied: 'PicDog does not have permission to complete this action.',
    network_timeout: 'The network request timed out. Try again later.',
    cancelled: 'Operation cancelled.',
    cancel_required: 'Cancel the current task before continuing.',
    batch_not_found: 'Current batch was not found. Choose images again.',
    asset_not_found: 'This image was not found. Choose it again.',
    asset_not_uploaded: 'Upload must finish before the link can be copied.',
    route_not_found: 'Upload target was not found. Choose or create one again.',
    route_delete_forbidden: 'This upload target cannot be deleted.',
    route_name_required: 'Upload target name is required.',
    route_public_base_url_required: 'CDN base URL is required for this upload target.',
    route_endpoint_required: 'Endpoint is required for this upload target.',
    route_bucket_required: 'Bucket is required for this upload target.',
    route_repository_required: 'GitHub repository is required for this upload target.',
    route_local_folder_required: 'Local folder is required for this upload target.',
    github_repository_invalid: 'Use owner/repo or a GitHub repository URL.',
    github_enterprise_base_url_required: 'GitHub Enterprise targets need a Raw or CDN base URL.',
    route_provider_unsupported: 'This upload provider is not supported yet.',
    custom_api_method_invalid: 'Custom API method must be POST, PUT, or PATCH.',
    custom_api_body_invalid: 'Custom API body must be binary or multipart.',
    invalid_history_scope: 'The history clear scope is invalid.',
    state_poisoned: 'App state is temporarily unavailable. Try again.',
    storage_error: 'Local data could not be saved or loaded. Try again.',
    clipboard_failed: 'Could not write to the clipboard. Copy manually.',
    database_unavailable: 'Local database is unavailable. Restart PicDog and try again.',
    unknown: 'The operation failed. Check the settings and try again.'
  },
  'ja-JP': {
    unsupported_format: 'この画像形式はまだサポートされていません。',
    file_not_found: '元のファイルが見つかりません。この Mac に残っているか確認してください。',
    encoder_missing: 'ローカルエンコーダが不足しています。設定でインストールするかフォルダを選択してください。',
    compression_failed: '圧縮に失敗しました。画像ファイルまたは圧縮設定を確認してください。',
    upload_disabled: 'アップロード機能が無効です。先に CDN 設定で有効にしてください。',
    upload_failed: 'アップロードに失敗しました。アップロード先の設定を確認してください。',
    credential_missing: 'アップロード認証情報がありません。先に CDN 設定で保存してください。',
    permission_denied: 'この操作を完了する権限がありません。',
    network_timeout: 'ネットワーク要求がタイムアウトしました。後でもう一度お試しください。',
    cancelled: '操作をキャンセルしました。',
    cancel_required: '続行する前に現在のタスクをキャンセルしてください。',
    batch_not_found: '現在のバッチが見つかりません。画像を選択し直してください。',
    asset_not_found: 'この画像が見つかりません。選択し直してください。',
    asset_not_uploaded: 'アップロード完了後にリンクをコピーできます。',
    route_not_found: 'アップロード先が見つかりません。選択または作成し直してください。',
    route_delete_forbidden: 'このアップロード先は削除できません。',
    route_name_required: 'アップロード先名を入力してください。',
    route_public_base_url_required: 'このアップロード先には CDN URL が必要です。',
    route_endpoint_required: 'このアップロード先には Endpoint が必要です。',
    route_bucket_required: 'このアップロード先には Bucket が必要です。',
    route_repository_required: 'このアップロード先には GitHub リポジトリが必要です。',
    route_local_folder_required: 'このアップロード先にはローカルフォルダが必要です。',
    github_repository_invalid: 'owner/repo または GitHub リポジトリ URL を入力してください。',
    github_enterprise_base_url_required: 'GitHub Enterprise には Raw または CDN URL が必要です。',
    route_provider_unsupported: 'このアップロード先の種類はまだサポートされていません。',
    custom_api_method_invalid: 'Custom API のメソッドは POST、PUT、PATCH のみ対応しています。',
    custom_api_body_invalid: 'Custom API の本文は binary または multipart のみ対応しています。',
    invalid_history_scope: '履歴削除の範囲が無効です。',
    state_poisoned: 'アプリの状態を一時的に利用できません。もう一度お試しください。',
    storage_error: 'ローカルデータを保存または読み込めません。もう一度お試しください。',
    clipboard_failed: 'クリップボードに書き込めません。手動でコピーしてください。',
    database_unavailable: 'ローカルデータベースを利用できません。PicDog を再起動してお試しください。',
    unknown: '操作に失敗しました。設定を確認してもう一度お試しください。'
  }
};

const uiMessages: Record<Locale, Record<UiMessageKey, string>> = {
  'zh-CN': {
    operationCancelled: '操作已取消。',
    operationFailed: '操作失败，请检查设置后重试。',
    noUploadTarget: '请先在设置里新建上传目标。',
    partialUploadFailed: '部分图片上传失败，请检查目标配置后重试。',
    routeTestSuccess: '上传目标连接成功，已完成一次测试上传。'
  },
  'en-US': {
    operationCancelled: 'Operation cancelled.',
    operationFailed: 'The operation failed. Check the settings and try again.',
    noUploadTarget: 'Create an upload target in Settings first.',
    partialUploadFailed: 'Some images failed to upload. Check the target settings and try again.',
    routeTestSuccess: 'Upload target is reachable. Test upload completed.'
  },
  'ja-JP': {
    operationCancelled: '操作をキャンセルしました。',
    operationFailed: '操作に失敗しました。設定を確認してもう一度お試しください。',
    noUploadTarget: '先に設定でアップロード先を作成してください。',
    partialUploadFailed: '一部の画像をアップロードできませんでした。送信先の設定を確認してください。',
    routeTestSuccess: 'アップロード先に接続できました。テストアップロードが完了しました。'
  }
};

function isSafeError(error: unknown): error is SafeError {
  return Boolean(
    error &&
      typeof error === 'object' &&
      'code' in error &&
      typeof (error as { code?: unknown }).code === 'string'
  );
}

export function localizedErrorMessage(error: unknown, locale: Locale): string {
  if (isSafeError(error)) {
    const code = error.code in errorMessages[locale] ? error.code : 'unknown';
    return errorMessages[locale][code] ?? uiMessages[locale].operationFailed;
  }

  if (locale === 'zh-CN' && typeof error === 'object' && error && 'message' in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === 'string' && message.trim()) return message;
  }

  return uiMessages[locale].operationFailed;
}

export function localizedUiMessage(key: UiMessageKey, locale: Locale): string {
  return uiMessages[locale][key];
}

export function localizedRouteTestMessage(result: RouteTestResult, locale: Locale): string {
  if (result.messageCode === 'route_test_success' || result.ok) {
    return localizedUiMessage('routeTestSuccess', locale);
  }

  if (result.messageCode) {
    return localizedErrorMessage(
      {
        code: result.messageCode,
        message: result.message,
        recoverable: true,
        retryable: false
      },
      locale
    );
  }

  return locale === 'zh-CN' && result.message.trim()
    ? result.message
    : localizedUiMessage('operationFailed', locale);
}
