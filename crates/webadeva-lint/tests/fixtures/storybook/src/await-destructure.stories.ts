import { userEvent, within } from '@storybook/test';

export default { title: 'AwaitDestructure' };

export const Primary = {
  play: async ({ canvasElement }) => {
    const { getByRole } = within(canvasElement);
    userEvent.click(getByRole('button'));
  },
};
