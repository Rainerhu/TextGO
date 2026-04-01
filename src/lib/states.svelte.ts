/**
 * Reactive class representing loading state.
 */
export class Loading {
  private loading = $state<boolean | null>(null);
  private timer: ReturnType<typeof setTimeout> | null = null;

  /**
   * Set the loading state to `false`, then set it to `true` after the specified delay.
   *
   * @param delay - milliseconds to wait before setting the loading state to `true`
   */
  start(delay: number = 500) {
    // clear any pending timer from a previous start() call
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.loading = false;
    if (delay <= 0) {
      this.loading = true;
    } else {
      this.timer = setTimeout(() => {
        if (this.loading === false) this.loading = true;
        this.timer = null;
      }, delay);
    }
  }

  /**
   * Set the loading state to `null`, indicating loading is complete.
   */
  end() {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.loading = null;
  }

  /**
   * Get current loading state.
   */
  get current() {
    return this.loading;
  }

  /**
   * Whether loading has already started.
   */
  get started() {
    return this.loading !== null;
  }

  /**
   * Whether the delay time has been reached.
   */
  get delayed() {
    return this.loading === true;
  }
}
