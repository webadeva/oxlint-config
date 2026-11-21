"use server";
import { Component } from "react";

export class Legacy extends Component {
  async UNSAFE_componentWillMount() {
    "use server";
    this.setState({ a: 1 });
  }
  UNSAFE_componentWillUpdate = async () => {
    "use server";
    this.setState({ b: 2 });
  };
  static async componentWillReceiveProps() {
    "use server";
  }
  UNSAFE_componentWillReceiveProps() {}
  componentDidMount() {
    this.setState({ c: 3 });
  }
  render() {
    return null;
  }
}

export async function action() {
  "use server";
}

export const other = async () => {
  "use server";
};
