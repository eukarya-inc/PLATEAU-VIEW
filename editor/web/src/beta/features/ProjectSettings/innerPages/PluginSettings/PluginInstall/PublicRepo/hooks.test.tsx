import { expect, test } from "vitest";

import { validateUrl } from "./hooks";

test("should validate url", () => {
  const wrongUrl1 = "https://google.com";
  const wrongUrl2 = "";
  const wrongUrl3 = "https://github.com";
  const correctUrl1 = "https://github.com/eukarya-inc/PLATEAU-VIEW";
  expect(validateUrl(wrongUrl1).result).toBe(false);
  expect(validateUrl(wrongUrl2).result).toBe(false);
  expect(validateUrl(wrongUrl3).result).toBe(false);
  expect(validateUrl(correctUrl1).result).toBe(true);
});
