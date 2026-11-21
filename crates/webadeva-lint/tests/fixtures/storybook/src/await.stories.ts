import { expect, userEvent, waitFor, within } from '@storybook/test';

export default { title: 'Await' };

export const Primary = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    userEvent.click(canvas.getByRole('button'));
    await userEvent.click(canvas.getByRole('button'));
    canvas.findByText('x');
    await canvas.findByText('x');
    expect(canvas.getByText('x')).toBeInTheDocument();
    await expect(1).toBe(1);
    expect(1).not.toBe(2);
    waitFor(() => expect(1).toBe(1));
    (userEvent.type(canvas.getByRole('textbox'), 'x'));
    await (userEvent.hover(canvas.getByRole('button')));
    canvas.findAllByRole('row');
    userEvent?.tab();
    canvas['findByLabelText']('a');
  },
};

export const Secondary = {
  play: async (context) => {
    await Primary.play(context);
    Primary.play(context);
    Primary.play();
    play();
    wait();
    within(context.canvasElement).findByRole('dialog')!;
    return userEvent.click(context.canvasElement);
  },
};

function helper() {
  return waitForElementToBeRemoved(() => null);
}
