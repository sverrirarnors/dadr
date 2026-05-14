import type { TaskResult } from '$lib/api';

export function lines(value: string): string[] {
  return value
    .split(/\r?\n|,/)
    .map((line) => line.trim())
    .filter(Boolean);
}

export function stringField(result: TaskResult, key: string): string {
  const value = result[key];
  return typeof value === 'string' ? value : '';
}

export function stringArrayField(result: TaskResult, key: string): string[] {
  const value = result[key];
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
}

export function objectArrayField(result: TaskResult, key: string): Record<string, unknown>[] {
  const value = result[key];
  return Array.isArray(value)
    ? value.filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === 'object')
    : [];
}

export function shortId(id: string): string {
  return id.replaceAll('-', '').slice(0, 12);
}
