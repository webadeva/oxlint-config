"use server";
import { Component } from "react";

export class Legacy extends Component {
  componentWillMount() {
    "use server";
    this.setState({ a: 1 });
  }
  componentWillUpdate = () => {
    "use server";
    this.setState({ b: 2 });
  };
  static componentWillReceiveProps() {
    "use server";
  }
  componentWillReceiveProps() {}
  componentDidMount() {
    this.setState({ c: 3 });
  }
  render() {
    return null;
  }
}

export function action() {
  "use server";
}

export const other = () => {
  "use server";
};
