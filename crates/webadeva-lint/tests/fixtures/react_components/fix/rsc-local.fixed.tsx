export function Page() {
  const arrow = async () => {
    "use server";
  };
  const typed = async <T,>(x: T) => {
    "use server";
    return x;
  };
  async function decl() {
    "use server";
  }
  const expr = async function () {
    "use server";
  };
  const obj = {
    async method() {
      "use server";
    },
    async *gen() {
      "use server";
    },
    get getter() {
      "use server";
      return 1;
    },
    set setter(v: number) {
      "use server";
    },
    prop: async function () {
      "use server";
    },
    async ["computed"]() {
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
  async method() {
    "use server";
  }
  static async staticMethod() {
    "use server";
  }
  public override async ["computed"]() {
    "use server";
  }
  async *gen() {
    "use server";
  }
  static async * /* star */ [Symbol.iterator] /* key */ () {
    "use server";
  }
  async #priv() {
    "use server";
  }
  get getter() {
    "use server";
    return 1;
  }
  field = async () => {
    "use server";
  };
  fn = async function () {
    "use server";
  };
  async ok() {
    "use server";
  }
}
