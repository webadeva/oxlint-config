class A {
  public static async foo<T>(a: T): Promise<T> {
    return a;
  }
  private bar?(b: number) {
    return b;
  }
  protected override baz() {
    return 1;
  }
  [Symbol.iterator]() {
    return null;
  }
  #priv() {
    return 2;
  }
  'quoted'() {
    return 3;
  }
  /* before */ qux() {
    return 4;
  }
  single() { return this.x; }
}
