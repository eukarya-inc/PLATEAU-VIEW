// TODO: remove MockedProvider after double-checking the test here is not necessary
// import { MockedProvider as MockedGqlProvider, MockedResponse } from "@apollo/client/testing";
import { render as rtlRender } from "@testing-library/react";
import { vitest } from "vitest";

// import { Provider as I18nProvider } from "../services/i18n";

// react-inlinesvg is not displayed in test.
// see detail: https://github.com/gilbarbara/react-inlinesvg/issues/145
vitest.mock("react-inlinesvg", () => {
  return {
    default: function InlineSvgMock(props: any) {
      return (
        <svg
          aria-label={props["aria-label"]}
          style={props.style}
          width={props.size}
          height={props.size}
        />
      );
    },
  };
});

const render = (
  ui: React.ReactElement,
  // queryMocks?: readonly MockedResponse<Record<string, any>>[],
  { ...renderOptions } = {},
) => {
  const Wrapper: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
    return <>{children}</>;
  };
  return rtlRender(ui, { wrapper: Wrapper, ...renderOptions });
};

export * from "@testing-library/react";

export { render };
