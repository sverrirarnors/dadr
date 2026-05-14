type ThemeChoice = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'dadr.theme';
const ORDER: ThemeChoice[] = ['system', 'light', 'dark'];

function read(): ThemeChoice {
  if (typeof localStorage === 'undefined') return 'system';
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === 'light' || stored === 'dark' ? stored : 'system';
}

function apply(choice: ThemeChoice) {
  if (typeof document === 'undefined') return;
  const html = document.documentElement;
  html.setAttribute('data-theme', choice);
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  if (choice === 'system' && prefersDark) {
    html.setAttribute('data-prefers-dark', '');
  } else {
    html.removeAttribute('data-prefers-dark');
  }
}

class ThemeStore {
  current = $state<ThemeChoice>(read());

  set(choice: ThemeChoice) {
    this.current = choice;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, choice);
    }
    apply(choice);
  }

  cycle() {
    const idx = ORDER.indexOf(this.current);
    this.set(ORDER[(idx + 1) % ORDER.length] ?? 'system');
  }
}

export const theme = new ThemeStore();

if (typeof window !== 'undefined') {
  apply(theme.current);
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  mq.addEventListener('change', () => apply(theme.current));
}
