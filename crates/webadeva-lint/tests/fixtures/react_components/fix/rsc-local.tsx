export function Page() {
  const arrow = () => {
    "use server";
  };
  const typed = <T,>(x: T) => {
    "use server";
    return x;
  };
  function decl() {
    "use server";
  }
  const expr = function () {
    "use server";
  };
  const obj = {
    method() {
      "use server";
    },
    *gen() {
      "use server";
    },
    get getter() {
      "use server";
      return 1;
    },
    set setter(v: number) {
      "use server";
    },
    prop: function () {
      "use server";
    },
    ["computed"]() {
      "use server";
    },
    already: async () => {
      "use server";
    },
  };
  return [arrow, typed, decl, expr, obj];
}

export class Actions {
  constructor() {
    "use server";
  }
  method() {
    "use server";
  }
  static staticMethod() {
    "use server";
  }
  public override ["computed"]() {
    "use server";
  }
  *gen() {
    "use server";
  }
  static * /* star */ [Symbol.iterator] /* key */ () {
    "use server";
  }
  #priv() {
    "use server";
  }
  get getter() {
    "use server";
    return 1;
  }
  field = () => {
    "use server";
  };
  fn = function () {
    "use server";
  };
  async ok() {
    "use server";
  }
}
