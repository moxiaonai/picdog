import { useEffect, useMemo, useState } from 'react';
import type {
  AppSettings,
  CustomApiBody,
  CustomApiMethod,
  Route,
  RouteCredentialInput,
  RouteCredentialStatus,
  RouteDraft,
  RouteTestResult,
  RouteProvider
} from '../../contracts/models';
import type { I18nKey } from '../../app/i18n';
import { localizedErrorMessage, localizedRouteTestMessage } from '../../app/errorMessages';

type RouteSettingsPopoverProps = {
  open: boolean;
  settings: AppSettings;
  routes: Route[];
  t: (key: I18nKey) => string;
  onUploadEnabledChange: (enabled: boolean) => void;
  onRouteChange: (routeId: string) => void;
  onSaveRoute: (draft: RouteDraft) => void | Promise<void>;
  onDeleteRoute: (routeId: string) => void | Promise<void>;
  onSaveCredential: (routeId: string, credential: RouteCredentialInput) => void | Promise<void>;
  onDeleteCredential: (routeId: string) => Promise<RouteCredentialStatus>;
  onGetCredentialStatus: (routeId: string) => Promise<RouteCredentialStatus>;
  onTestRoute: (routeId: string) => Promise<RouteTestResult>;
  onSelectDirectory: (title?: string) => Promise<string | null>;
  onClose: () => void;
};

const routeProviders: Array<{ value: RouteProvider; label: I18nKey }> = [
  { value: 's3', label: 'providerS3' },
  { value: 'aliyun_oss', label: 'providerAliyunOss' },
  { value: 'tencent_cos', label: 'providerTencentCos' },
  { value: 'qiniu', label: 'providerQiniu' },
  { value: 'github', label: 'providerGithub' },
  { value: 'local_folder', label: 'providerLocalFolder' },
  { value: 'custom_api', label: 'providerCustomApi' }
];

const defaultCacheControl = 'public, max-age=31536000, immutable';
const customApiMethods: CustomApiMethod[] = ['POST', 'PUT', 'PATCH'];
const customApiBodies: Array<{ value: CustomApiBody; label: I18nKey }> = [
  { value: 'binary', label: 'customApiBodyBinary' },
  { value: 'multipart', label: 'customApiBodyMultipart' }
];

function emptyRouteName() {
  return 'PicDog CDN';
}

function providerLabelKey(provider: RouteProvider): I18nKey {
  return routeProviders.find((item) => item.value === provider)?.label ?? 'providerS3';
}

function routeEndpointPlaceholder(provider: RouteProvider) {
  if (provider === 'github') return 'owner/repo';
  if (provider === 'aliyun_oss') return 'https://{bucket}.oss-cn-hangzhou.aliyuncs.com';
  if (provider === 'tencent_cos') return 'https://{bucket}.cos.ap-shanghai.myqcloud.com';
  if (provider === 'qiniu') return 'https://upload.qiniup.com';
  if (provider === 's3') return 'https://{bucket}.s3.amazonaws.com';
  if (provider === 'custom_api') return 'https://api.example.com/upload';
  return undefined;
}

function routeBucketPlaceholder(provider: RouteProvider) {
  if (provider === 'github') return 'main';
  if (provider === 'local_folder' || provider === 'custom_api') return undefined;
  if (provider === 'qiniu') return 'assets';
  return 'assets';
}

function routeRegionPlaceholder(provider: RouteProvider) {
  if (provider === 'aliyun_oss') return 'cn-hangzhou';
  if (provider === 'tencent_cos') return 'ap-shanghai';
  if (provider === 'qiniu') return 'https://rs.qiniu.com';
  return 'auto';
}

function routeEndpointLabel(provider: RouteProvider): I18nKey {
  if (provider === 'local_folder') return 'localFolderPath';
  if (provider === 'github') return 'routeGithubRepository';
  return 'routeEndpoint';
}

function routeBucketLabel(provider: RouteProvider): I18nKey {
  if (provider === 'github') return 'routeGithubBranch';
  return 'routeBucket';
}

function routeRegionLabel(provider: RouteProvider): I18nKey {
  if (provider === 'qiniu') return 'qiniuRsEndpoint';
  return 'routeRegion';
}

function routeBaseUrlLabel(provider: RouteProvider): I18nKey {
  if (provider === 'github') return 'routeGithubBaseUrl';
  return 'routeBaseUrl';
}

function usesTokenCredential(provider: RouteProvider) {
  return provider === 'custom_api' || provider === 'github';
}

export function RouteSettingsPopover({
  open,
  settings,
  routes,
  t,
  onUploadEnabledChange,
  onRouteChange,
  onSaveRoute,
  onDeleteRoute,
  onSaveCredential,
  onDeleteCredential,
  onGetCredentialStatus,
  onTestRoute,
  onSelectDirectory,
  onClose
}: RouteSettingsPopoverProps) {
  const activeRoute = useMemo(
    () => routes.find((route) => route.id === settings.defaultRouteId) ?? routes[0],
    [routes, settings.defaultRouteId]
  );

  const [isCreatingRoute, setIsCreatingRoute] = useState(false);
  const [provider, setProvider] = useState<RouteProvider>('s3');
  const [routeName, setRouteName] = useState('');
  const [endpoint, setEndpoint] = useState('');
  const [bucket, setBucket] = useState('');
  const [region, setRegion] = useState('auto');
  const [concurrency, setConcurrency] = useState(3);
  const [publicBaseUrl, setPublicBaseUrl] = useState('');
  const [cacheControl, setCacheControl] = useState(defaultCacheControl);
  const [pathTemplate, setPathTemplate] = useState('{yyyy}/{mm}/{hash}-{name}.{ext}');
  const [customLinkTemplate, setCustomLinkTemplate] = useState('{url}');
  const [accessKeyId, setAccessKeyId] = useState('');
  const [secretAccessKey, setSecretAccessKey] = useState('');
  const [sessionToken, setSessionToken] = useState('');
  const [apiToken, setApiToken] = useState('');
  const [customApiMethod, setCustomApiMethod] = useState<CustomApiMethod>('POST');
  const [customApiBody, setCustomApiBody] = useState<CustomApiBody>('binary');
  const [customApiFileField, setCustomApiFileField] = useState('file');
  const [customApiHeaders, setCustomApiHeaders] = useState('');
  const [customApiUrlPath, setCustomApiUrlPath] = useState('');
  const [routeStatus, setRouteStatus] = useState('');

  useEffect(() => {
    if (!open) return;

    if (isCreatingRoute) {
      setProvider('s3');
      setRouteName(emptyRouteName());
      setEndpoint('');
      setBucket('');
      setRegion('auto');
      setConcurrency(3);
      setPublicBaseUrl('');
      setCacheControl(defaultCacheControl);
      setPathTemplate('{yyyy}/{mm}/{hash}-{name}.{ext}');
      setCustomLinkTemplate('{url}');
      setAccessKeyId('');
      setSecretAccessKey('');
      setSessionToken('');
      setApiToken('');
      setCustomApiMethod('POST');
      setCustomApiBody('binary');
      setCustomApiFileField('file');
      setCustomApiHeaders('');
      setCustomApiUrlPath('url');
      setRouteStatus(t('saveTargetFirst'));
      return;
    }

    if (!activeRoute) {
      setProvider('s3');
      setRouteName(emptyRouteName());
      setEndpoint('');
      setBucket('');
      setRegion('auto');
      setConcurrency(3);
      setPublicBaseUrl('');
      setCacheControl(defaultCacheControl);
      setPathTemplate('{yyyy}/{mm}/{hash}-{name}.{ext}');
      setCustomLinkTemplate('{url}');
      setAccessKeyId('');
      setSecretAccessKey('');
      setSessionToken('');
      setApiToken('');
      setCustomApiMethod('POST');
      setCustomApiBody('binary');
      setCustomApiFileField('file');
      setCustomApiHeaders('');
      setCustomApiUrlPath('url');
      setRouteStatus(t('noUploadTarget'));
      return;
    }

    let disposed = false;
    setRouteName(activeRoute.name);
    setProvider(activeRoute.provider);
    setEndpoint(activeRoute.endpoint ?? '');
    setBucket(activeRoute.bucket ?? '');
    setRegion(activeRoute.region ?? 'auto');
    setConcurrency(activeRoute.concurrency);
    setPublicBaseUrl(activeRoute.publicBaseUrl);
    setCacheControl(activeRoute.cacheControl || defaultCacheControl);
    setPathTemplate(activeRoute.pathTemplate);
    setCustomLinkTemplate(activeRoute.customLinkTemplate ?? '{url}');
    setAccessKeyId('');
    setSecretAccessKey('');
    setSessionToken('');
    setApiToken('');
    setCustomApiMethod(activeRoute.customApiMethod ?? 'POST');
    setCustomApiBody(activeRoute.customApiBody ?? 'binary');
    setCustomApiFileField(activeRoute.customApiFileField ?? 'file');
    setCustomApiHeaders(activeRoute.customApiHeaders ?? '');
    setCustomApiUrlPath(activeRoute.customApiUrlPath ?? 'url');
    setRouteStatus('');

    void onGetCredentialStatus(activeRoute.id)
      .then((status) => {
        if (!disposed) {
          setRouteStatus(status.configured ? t('credentialReady') : t('credentialMissing'));
        }
      })
      .catch(() => {
        if (!disposed) {
          setRouteStatus(t('credentialMissing'));
        }
      });

    return () => {
      disposed = true;
    };
  }, [activeRoute, isCreatingRoute, onGetCredentialStatus, open, t]);

  if (!open) return null;

  const changeProvider = (nextProvider: RouteProvider) => {
    const currentName = routeName.trim();
    const looksLikeDefaultName =
      !currentName ||
      currentName === emptyRouteName() ||
      routeProviders.some((item) => currentName === t(item.label));

    setProvider(nextProvider);
    setRegion(routeRegionPlaceholder(nextProvider));

    if ((isCreatingRoute || !activeRoute) && looksLikeDefaultName) {
      setRouteName(t(providerLabelKey(nextProvider)));
    }

    if (isCreatingRoute || !activeRoute) {
      setEndpoint('');
      setBucket('');
      setPublicBaseUrl('');
      setCacheControl(defaultCacheControl);
      setPathTemplate('{yyyy}/{mm}/{hash}-{name}.{ext}');
    }
  };

  const saveTarget = async () => {
    try {
      await onSaveRoute({
        id: isCreatingRoute ? undefined : activeRoute?.id,
        name: routeName,
        provider,
        endpoint,
        bucket,
        region,
        pathTemplate,
        publicBaseUrl,
        cacheControl,
        defaultCopyFormat: settings.copyFormat,
        customLinkTemplate,
        customApiMethod: provider === 'custom_api' ? customApiMethod : undefined,
        customApiBody: provider === 'custom_api' ? customApiBody : undefined,
        customApiFileField: provider === 'custom_api' ? customApiFileField : undefined,
        customApiHeaders: provider === 'custom_api' ? customApiHeaders : undefined,
        customApiUrlPath: provider === 'custom_api' ? customApiUrlPath : undefined,
        concurrency: Math.min(8, Math.max(1, concurrency))
      });
      setIsCreatingRoute(false);
      setRouteStatus(t('targetSaved'));
    } catch (error) {
      setRouteStatus(localizedErrorMessage(error, settings.locale));
    }
  };

  const saveCredential = async () => {
    if (!activeRoute || isCreatingRoute) return;

    try {
      await onSaveCredential(
        activeRoute.id,
        usesTokenCredential(provider)
          ? { apiToken }
          : provider === 'local_folder'
            ? {}
            : { accessKeyId, secretAccessKey, sessionToken }
      );
      setApiToken('');
      setSecretAccessKey('');
      setSessionToken('');
      setRouteStatus(t('credentialReady'));
    } catch (error) {
      setRouteStatus(localizedErrorMessage(error, settings.locale));
    }
  };

  const deleteCredential = async () => {
    if (!activeRoute || isCreatingRoute || provider === 'local_folder') return;

    try {
      const status = await onDeleteCredential(activeRoute.id);
      setAccessKeyId('');
      setSecretAccessKey('');
      setSessionToken('');
      setApiToken('');
      setRouteStatus(status.configured ? t('credentialReady') : t('credentialMissing'));
    } catch (error) {
      setRouteStatus(localizedErrorMessage(error, settings.locale));
    }
  };

  const testTarget = async () => {
    if (!activeRoute || isCreatingRoute) return;

    try {
      const result = await onTestRoute(activeRoute.id);
      setRouteStatus(localizedRouteTestMessage(result, settings.locale));
    } catch (error) {
      setRouteStatus(localizedErrorMessage(error, settings.locale));
    }
  };

  const chooseLocalFolder = async () => {
    try {
      const path = await onSelectDirectory(t('chooseLocalPublishFolderTitle'));
      if (path) {
        setEndpoint(path);
      }
    } catch (error) {
      setRouteStatus(localizedErrorMessage(error, settings.locale));
    }
  };

  const deleteTarget = async () => {
    if (!activeRoute || isCreatingRoute) return;

    try {
      await onDeleteRoute(activeRoute.id);
      setRouteStatus(t('routeDeleted'));
    } catch (error) {
      setRouteStatus(localizedErrorMessage(error, settings.locale));
    }
  };

  return (
    <section className="popover route-settings-popover" aria-label={t('cdnSettings')}>
      <div className="popover-head">
        <strong>{t('cdnSettings')}</strong>
        <button className="close-button" type="button" onClick={onClose} aria-label="Close">
          ×
        </button>
      </div>

      <div className="popover-body">
        <label className="settings-row upload-feature-row">
          <span>
            <strong>{t('uploadFeature')}</strong>
            <small>{t('uploadFeatureDesc')}</small>
          </span>
          <input
            className="upload-feature-switch"
            type="checkbox"
            role="switch"
            checked={settings.uploadEnabled}
            aria-label={t('uploadFeature')}
            onChange={(event) => onUploadEnabledChange(event.currentTarget.checked)}
          />
        </label>

        <label className="settings-row">
          <span>
            <strong>{t('uploadTarget')}</strong>
            <small>{t('manualUploadTarget')}</small>
          </span>
          <div className="target-picker">
            <select
              value={isCreatingRoute ? '' : settings.defaultRouteId ?? ''}
              disabled={isCreatingRoute}
              onChange={(event) => onRouteChange(event.target.value)}
            >
              {routes.length === 0 && (
                <option value="">{t('noUploadTarget')}</option>
              )}
              {routes.map((route) => (
                <option key={route.id} value={route.id}>
                  {route.name}
                </option>
              ))}
            </select>
            <button className="soft-button" type="button" onClick={() => setIsCreatingRoute(true)}>
              {t('newTarget')}
            </button>
          </div>
        </label>

        {routes.length === 0 && !isCreatingRoute && (
          <div className="first-route-guide">
            <div>
              <strong>{t('firstRouteTitle')}</strong>
              <small>{t('firstRouteDesc')}</small>
            </div>
            <ol>
              <li>{t('firstRouteStepProvider')}</li>
              <li>{t('firstRouteStepPath')}</li>
              <li>{t('firstRouteStepCredential')}</li>
            </ol>
            <button className="primary-button compact-primary" type="button" onClick={() => setIsCreatingRoute(true)}>
              {t('newTarget')}
            </button>
          </div>
        )}

        <div className="route-editor">
          <div className="route-editor-head">
            <span>
              <strong>{isCreatingRoute || !activeRoute ? t('newTarget') : t('uploadTarget')}</strong>
              <small>{routeStatus || t('credentialMissing')}</small>
            </span>
            {!isCreatingRoute && activeRoute && (
              <button type="button" className="ghost-danger" onClick={deleteTarget}>
                {t('deleteTarget')}
              </button>
            )}
          </div>

          <div className="settings-grid">
            <label>
              <span>{t('routeProvider')}</span>
              <select value={provider} onChange={(event) => changeProvider(event.target.value as RouteProvider)}>
                {routeProviders.map((item) => (
                  <option key={item.value} value={item.value}>
                    {t(item.label)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>{t('routeName')}</span>
              <input value={routeName} onChange={(event) => setRouteName(event.target.value)} />
            </label>
            <label>
              <span>{t(routeEndpointLabel(provider))}</span>
              <div className="input-with-button">
                <input
                  value={endpoint}
                  placeholder={routeEndpointPlaceholder(provider)}
                  onChange={(event) => setEndpoint(event.target.value)}
                />
                {provider === 'local_folder' && (
                  <button type="button" className="soft-button" onClick={chooseLocalFolder}>
                    {t('chooseFolder')}
                  </button>
                )}
              </div>
            </label>
            <label>
              <span>{t(routeBucketLabel(provider))}</span>
              <input
                value={bucket}
                placeholder={routeBucketPlaceholder(provider)}
                onChange={(event) => setBucket(event.target.value)}
              />
            </label>
            {provider !== 'github' && (
              <label>
                <span>{t(routeRegionLabel(provider))}</span>
                <input
                  value={region}
                  placeholder={routeRegionPlaceholder(provider)}
                  onChange={(event) => setRegion(event.target.value)}
                />
              </label>
            )}
            <label>
              <span>{t('routeConcurrency')}</span>
              <input
                type="number"
                min="1"
                max="8"
                value={concurrency}
                onChange={(event) => setConcurrency(Math.min(8, Math.max(1, Number(event.target.value) || 1)))}
              />
            </label>
            <label className="wide">
              <span>{t(routeBaseUrlLabel(provider))}</span>
              <input
                value={publicBaseUrl}
                placeholder={provider === 'github' ? 'Auto raw.githubusercontent.com' : undefined}
                onChange={(event) => setPublicBaseUrl(event.target.value)}
              />
            </label>
            <label className="wide">
              <span>{t('routePathTemplate')}</span>
              <input value={pathTemplate} onChange={(event) => setPathTemplate(event.target.value)} />
              <small className="field-help">{t('routePathTemplateHelp')}</small>
            </label>
            <label className="wide">
              <span>{t('routeCacheControl')}</span>
              <input
                value={cacheControl}
                placeholder={defaultCacheControl}
                onChange={(event) => setCacheControl(event.target.value)}
              />
            </label>
            {settings.copyFormat === 'custom' && (
              <label className="wide">
                <span>{t('customTemplate')}</span>
                <input
                  value={customLinkTemplate}
                  placeholder="<Image src=&quot;{url}&quot; alt=&quot;{alt}&quot; />"
                  onChange={(event) => setCustomLinkTemplate(event.target.value)}
                />
              </label>
            )}
            {provider === 'custom_api' && (
              <>
                <label>
                  <span>{t('customApiMethod')}</span>
                  <select value={customApiMethod} onChange={(event) => setCustomApiMethod(event.target.value as CustomApiMethod)}>
                    {customApiMethods.map((method) => (
                      <option key={method} value={method}>
                        {method}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>{t('customApiBody')}</span>
                  <select value={customApiBody} onChange={(event) => setCustomApiBody(event.target.value as CustomApiBody)}>
                    {customApiBodies.map((item) => (
                      <option key={item.value} value={item.value}>
                        {t(item.label)}
                      </option>
                    ))}
                  </select>
                </label>
                {customApiBody === 'multipart' && (
                  <label>
                    <span>{t('customApiFileField')}</span>
                    <input value={customApiFileField} onChange={(event) => setCustomApiFileField(event.target.value)} />
                  </label>
                )}
                <label>
                  <span>{t('customApiUrlPath')}</span>
                  <input
                    value={customApiUrlPath}
                    placeholder="data.url"
                    onChange={(event) => setCustomApiUrlPath(event.target.value)}
                  />
                </label>
                <label className="wide">
                  <span>{t('customApiHeaders')}</span>
                  <textarea
                    value={customApiHeaders}
                    placeholder={t('customApiHeadersPlaceholder')}
                    onChange={(event) => setCustomApiHeaders(event.target.value)}
                  />
                </label>
              </>
            )}
            {provider === 'local_folder' ? null : usesTokenCredential(provider) ? (
              <label className="wide">
                <span>{provider === 'github' ? t('githubToken') : t('apiToken')}</span>
                <input
                  type="password"
                  value={apiToken}
                  onChange={(event) => setApiToken(event.target.value)}
                />
              </label>
            ) : (
              <>
                <label>
                  <span>{t('accessKeyId')}</span>
                  <input value={accessKeyId} onChange={(event) => setAccessKeyId(event.target.value)} />
                </label>
                <label>
                  <span>{t('secretAccessKey')}</span>
                  <input
                    type="password"
                    value={secretAccessKey}
                    onChange={(event) => setSecretAccessKey(event.target.value)}
                  />
                </label>
                <label className="wide">
                  <span>{t('sessionToken')}</span>
                  <input
                    type="password"
                    value={sessionToken}
                    onChange={(event) => setSessionToken(event.target.value)}
                  />
                </label>
              </>
            )}
          </div>

          <div className="route-actions">
            <small>{routeStatus || t('credentialMissing')}</small>
            <div>
              <button type="button" className="soft-button" onClick={testTarget} disabled={!activeRoute || isCreatingRoute}>
                {t('testTarget')}
              </button>
              <button type="button" className="soft-button" onClick={saveCredential} disabled={!activeRoute || isCreatingRoute}>
                {t('saveCredential')}
              </button>
              {provider !== 'local_folder' && (
                <button type="button" className="soft-button danger-soft" onClick={deleteCredential} disabled={!activeRoute || isCreatingRoute}>
                  {t('deleteCredential')}
                </button>
              )}
              <button type="button" className="primary-button compact-primary" onClick={saveTarget}>
                {t('saveTarget')}
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
