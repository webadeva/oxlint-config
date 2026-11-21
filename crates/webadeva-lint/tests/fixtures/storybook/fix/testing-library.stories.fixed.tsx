import { screen } from 'storybook/test';
import { userEvent } from 'storybook/test';
import { dom, type Matcher, within } from "storybook/test";
import * as tl from 'storybook/test';
import 'storybook/test';

export default { title: 'TL' };

export const Primary = { args: { screen, userEvent, dom, within, tl } };
