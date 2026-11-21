class A {
  public static foo = async <T>(a: T): Promise<T> => a;
  private bar? = (b: number) => b;
  protected override baz = () => 1;
  [Symbol.iterator] = () => null;
  #priv = () => 2;
  'quoted' = () => 3;
  /* before */ qux = () => 4;
  single() { return this.x; }
}
