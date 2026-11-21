import { userEvent } from '@storybook/test';

export default { title: 'Await' };

export const Method = {
  play(context) {
    Other.play(context);
  },
};

export const MethodTyped = {
  play<T>(context: T) {
    (userEvent.tab());
  },
};
