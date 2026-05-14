class PaletteStore {
  open = $state(false);

  toggle() {
    this.open = !this.open;
  }

  show() {
    this.open = true;
  }

  hide() {
    this.open = false;
  }
}

export const palette = new PaletteStore();
