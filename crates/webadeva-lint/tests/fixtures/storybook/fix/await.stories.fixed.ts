import { expect, userEvent, within } from '@storybook/test';

export default { title: 'Await' };

export const Arrow = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole('button'));
    await canvas.findByText('x');
    await expect(canvas.getByText('x')).toBeInTheDocument();
  },
};

export const FnExpr = {
  play: async function ({ canvasElement }) {
    await waitFor(() => expect(1).toBe(1));
  },
};

export async function helper() {
  const x = await userEvent.type(document.body, 'x');
  return x;
}

export const Nested = {
  play: async () => {
    [1].forEach(async (n) => {
      await userEvent.click(n);
    });
    await userEvent.dblClick(document.body);
  },
};

export const Generic = {
  play: async <T,>(ctx: T) => { await userEvent.hover(ctx); },
};

await wait();
