import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import dom, { type   Matcher,
  within } from "@testing-library/dom";
import * as tl from '@testing-library/react';
import '@testing-library/jest-dom';

export default { title: 'TL' };

export const Primary = { args: { screen, userEvent, dom, within, tl } };
