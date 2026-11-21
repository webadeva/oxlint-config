import { userEvent } from '@storybook/test';

export default { title: 'Await' };

export const Method = {
  playasync (context) {
    await Other.play(context);
  },
};

export const MethodTyped = {
  playasync <T>(context: T) {
    (await userEvent.tab());
  },
};
