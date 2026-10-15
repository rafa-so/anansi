// Edição de ambientes: linhas de variáveis e de segredos (Seção 13.5).

export type VarRow = { key: string; value: string };

/** `value === null`: segredo existente, ainda mascarado e sem alteração. */
export type SecretRow = { key: string; originalKey: string | null; value: string | null; revealed: boolean };

const NAME = /^[A-Za-z0-9_.-]+$/;

/** Problemas de validação (RN-07): nomes inválidos, repetidos ou em variável e segredo ao mesmo tempo. */
export function validateRows(vars: VarRow[], secrets: SecretRow[]): string[] {
  const problems: string[] = [];
  const seen = new Map<string, string>();
  const check = (key: string, kind: string) => {
    if (!key) return;
    if (!NAME.test(key)) problems.push(`Nome inválido: "${key}" (use letras, números, "_", "." e "-").`);
    const previous = seen.get(key);
    if (previous) problems.push(`"${key}" aparece mais de uma vez (${previous} e ${kind}).`);
    else seen.set(key, kind);
  };
  vars.forEach((v) => check(v.key.trim(), 'variável'));
  secrets.forEach((s) => check(s.key.trim(), 'segredo'));
  return problems;
}

export function variablesPayload(rows: VarRow[]): Record<string, string> {
  return Object.fromEntries(rows.filter((r) => r.key.trim()).map((r) => [r.key.trim(), r.value]));
}

/**
 * Payload de `env_save_secrets`: `null` mantém o valor guardado. Um segredo
 * renomeado sem valor novo precisa do valor antigo (`reveal`).
 */
export async function secretsPayload(
  rows: SecretRow[],
  reveal: (key: string) => Promise<string>,
): Promise<Record<string, string | null>> {
  const payload: Record<string, string | null> = {};
  for (const row of rows) {
    const key = row.key.trim();
    if (!key) continue;
    if (row.value !== null) payload[key] = row.value;
    else if (row.originalKey === key) payload[key] = null;
    else if (row.originalKey) payload[key] = await reveal(row.originalKey);
  }
  return payload;
}
