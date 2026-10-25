import { render, screen } from "@testing-library/react";
import App from "./App";

describe("app shell", () => {
  it("renders the three navigation items", () => {
    render(<App />);

    const navigation = screen.getByRole("navigation", {
      name: "Основная навигация",
    });
    expect(navigation).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Файлы/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Транскрипция/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Экспорт/ }),
    ).toBeInTheDocument();
  });
});
