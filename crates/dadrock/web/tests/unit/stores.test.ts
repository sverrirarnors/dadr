import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { toasts } from '../../src/lib/stores/toast.svelte';

describe('toasts store', () => {
  beforeEach(() => {
    toasts.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('adds and removes toasts', () => {
    const id = toasts.push({ severity: 'info', message: 'hello' });
    expect(toasts.list.length).toBe(1);
    toasts.dismiss(id);
    expect(toasts.list.length).toBe(0);
  });

  it('assigns unique ids', () => {
    const a = toasts.push({ severity: 'info', message: 'a' });
    const b = toasts.push({ severity: 'info', message: 'b' });
    expect(a).not.toBe(b);
  });

  it('automatically dismisses toasts after three seconds', () => {
    vi.useFakeTimers();
    toasts.push({ severity: 'success', message: 'accepted candidate' });
    expect(toasts.list.length).toBe(1);
    vi.advanceTimersByTime(2999);
    expect(toasts.list.length).toBe(1);
    vi.advanceTimersByTime(1);
    expect(toasts.list.length).toBe(0);
  });
});
