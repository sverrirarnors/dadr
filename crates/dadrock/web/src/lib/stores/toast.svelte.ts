export type ToastSeverity = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
  id: number;
  severity: ToastSeverity;
  message: string;
  detail?: string;
}

let nextId = 1;
const DEFAULT_TIMEOUT_MS = 3000;

class ToastStore {
  list = $state<Toast[]>([]);

  push(toast: Omit<Toast, 'id'>): number {
    const id = nextId++;
    this.list.push({ id, ...toast });
    window.setTimeout(() => this.dismiss(id), DEFAULT_TIMEOUT_MS);
    return id;
  }

  dismiss(id: number) {
    const idx = this.list.findIndex((toast) => toast.id === id);
    if (idx !== -1) this.list.splice(idx, 1);
  }

  clear() {
    this.list.splice(0, this.list.length);
  }
}

export const toasts = new ToastStore();
