// Tipos espelhando os modelos do Rust (Seção 4 e 5 da spec).

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE' | 'HEAD' | 'OPTIONS';

export const HTTP_METHODS: HttpMethod[] = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'];

export type KeyValue = { key: string; value: string; enabled: boolean };

export type FormFieldType = 'text' | 'file';

/** Campo do body de formulário. Sem `type` é texto; no `file`, o `value` é o caminho do arquivo. */
export type FormField = KeyValue & { type?: FormFieldType };

export type Body =
  | { type: 'none' }
  | { type: 'json'; content: string }
  | { type: 'text'; content: string }
  | { type: 'form-urlencoded'; fields: FormField[] };

export type BodyType = Body['type'];

export type Settings = {
  verifyTls?: boolean;
  timeoutMs?: number;
  followRedirects?: boolean;
};

export type RequestFile = {
  schemaVersion: number;
  name: string;
  method: HttpMethod;
  url: string;
  query?: KeyValue[];
  headers?: KeyValue[];
  body?: Body;
  settings?: Settings;
};

export type WorkspaceInfo = {
  path: string;
  name: string;
  environments: string[];
  gitignoreOk: boolean;
};

export type TreeNode =
  | {
      kind: 'collection';
      path: string;
      name: string;
      description?: string;
      children: TreeNode[];
      error?: string;
    }
  | { kind: 'folder'; path: string; name: string; children: TreeNode[] }
  | {
      kind: 'request';
      path: string;
      name: string;
      method: string;
      url: string;
      error?: string;
      readOnly?: boolean;
    };

export type AppError = { code: string; message: string };

export function isAppError(e: unknown): e is AppError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e;
}

/** Mensagem pronta para exibir, para qualquer erro vindo do backend ou do JS. */
export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}

export type HttpResponse = {
  status: number;
  statusText: string;
  headers: [string, string][];
  body: string;
  bodyEncoding: 'utf8' | 'binary';
  bodyTruncated: boolean;
  sizeBytes: number;
  elapsedMs: number;
  tlsVerified: boolean;
  unresolved: string[];
  url: string;
};

export type EnvView = {
  name: string;
  variables: Record<string, string>;
  secretNames: string[];
  warnings: string[];
};

export type CurlFormat = 'bash' | 'powershell';

export type GlobalPrefs = { defaultVerifyTls: boolean; defaultTimeoutMs: number };
