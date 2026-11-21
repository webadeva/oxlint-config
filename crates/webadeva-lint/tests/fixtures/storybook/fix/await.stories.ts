import { expect, userEvent, within } from '@storybook/test';

export default { title: 'Await' };

export const Arrow = {
  play: ({ canvasElement }) => {
    const canvas = within(canvasElement);
    userEvent.click(canvas.getByRole('button'));
    canvas.findByText('x');
    expect(canvas.getByText('x')).toBeInTheDocument();
  },
};

export const FnExpr = {
  play: function ({ canvasElement }) {
    waitFor(() => expect(1).toBe(1));
  },
};

export function helper() {
  const x = userEvent.type(document.body, 'x');
  return x;
}

export const Nested = {
  play: async () => {
    [1].forEach((n) => {
      userEvent.click(n);
    });
    userEvent.dblClick(document.body);
  },
};

export const Generic = {
  play: <T,>(ctx: T) => { userEvent.hover(ctx); },
};

wait();
