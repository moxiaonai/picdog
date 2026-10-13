-- PicDog initial local schema.
-- Sensitive credentials are intentionally excluded. Store only credential_ref here;
-- the secret value belongs in the system Keychain / Credential Manager / Secret Service.

create table if not exists settings (
  key text primary key,
  value text not null,
  updated_at text not null
);

create table if not exists routes (
  id text primary key,
  name text not null,
  provider text not null,
  credential_ref text not null,
  endpoint text,
  bucket text,
  region text,
  path_template text not null,
  public_base_url text not null,
  cache_control text not null default 'public, max-age=31536000, immutable',
  default_copy_format text not null,
  custom_link_template text,
  custom_api_method text,
  custom_api_body text,
  custom_api_file_field text,
  custom_api_headers text,
  custom_api_url_path text,
  concurrency integer not null default 3,
  created_at text not null,
  updated_at text not null
);

create table if not exists batches (
  id text primary key,
  status text not null,
  asset_count integer not null default 0,
  uploaded_count integer not null default 0,
  source_total_size integer not null default 0,
  output_total_size integer,
  route_id text,
  route_name text,
  copy_format text,
  cleared_at text,
  created_at text not null,
  updated_at text not null
);

create table if not exists assets (
  id text primary key,
  batch_id text not null,
  source_path text not null,
  output_path text,
  file_name text not null,
  source_size integer not null,
  output_size integer,
  saved_ratio integer,
  output_format text,
  width integer,
  height integer,
  mime text,
  compress_status text not null,
  upload_status text not null,
  remote_key text,
  public_url text,
  route_id text,
  route_name text,
  copied_at text,
  uploaded_at text,
  history_hidden integer not null default 0,
  output_deleted_at text,
  error_code text,
  error_message text,
  cleared_at text,
  created_at text not null,
  updated_at text not null,
  foreign key(batch_id) references batches(id)
);

create index if not exists idx_batches_cleared_created on batches(cleared_at, created_at);
create index if not exists idx_assets_batch_id on assets(batch_id);
create index if not exists idx_assets_cleared_created on assets(cleared_at, created_at);
create index if not exists idx_assets_public_url on assets(public_url);
