import { render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it } from "vitest";
import { Badge } from "./Badge";
import { Button } from "./Button";
import { Card } from "./Card";
import { Field } from "./Field";
import { Row } from "./Row";
import { Stack } from "./Stack";

describe("primitives", () => {
  it("renders a button of type button with its variant and the screen's own class", () => {
    render(
      <Button variant="primary" className="about__close">
        Cerrar
      </Button>,
    );

    const button = screen.getByRole("button", { name: "Cerrar" });
    expect(button).toHaveAttribute("type", "button");
    expect(button).toHaveClass("rf-btn", "rf-btn--primary", "about__close");
  });

  it("lets a button be a submit and hands its ref over", () => {
    const ref = createRef<HTMLButtonElement>();
    render(
      <Button type="submit" ref={ref}>
        Enviar
      </Button>,
    );

    expect(ref.current).toBe(screen.getByRole("button", { name: "Enviar" }));
    expect(ref.current).toHaveAttribute("type", "submit");
    expect(ref.current).not.toHaveClass("rf-btn--primary");
  });

  it("rejects a variant that does not exist", () => {
    // @ts-expect-error la variante no existe
    render(<Button variant="danger">x</Button>);
    // @ts-expect-error el hueco no existe
    render(<Stack gap="xl">x</Stack>);
    // @ts-expect-error el hueco no existe
    render(<Row gap="md">x</Row>);
    // @ts-expect-error la variante no existe
    render(<Badge variant="secondary">x</Badge>);
  });

  it("maps gaps, elevation and badge variant to their classes", () => {
    render(
      <>
        <Stack gap="md" data-testid="stack" />
        <Row gap="xs" data-testid="row" />
        <Card elevated data-testid="card" />
        <Badge variant="primary" data-testid="badge" />
        <Field data-testid="field" />
      </>,
    );

    expect(screen.getByTestId("stack")).toHaveClass("rf-stack", "rf-gap-md");
    expect(screen.getByTestId("row")).toHaveClass("rf-row", "rf-gap-xs");
    expect(screen.getByTestId("card")).toHaveClass("rf-card", "rf-card--elevated");
    expect(screen.getByTestId("badge")).toHaveClass("rf-badge", "rf-badge--primary");
    expect(screen.getByTestId("field")).toHaveClass("rf-field");
  });

  it("renders a div by default and the given element with as, keeping the classes", () => {
    render(
      <>
        <Row data-testid="row" gap="xs" />
        <Stack data-testid="stack" gap="md" />
        <Card data-testid="card" />
        <Row as="li" data-testid="row-li" gap="xs" />
        <Stack as="ul" data-testid="stack-ul" gap="md" />
        <Card as="li" data-testid="card-li" />
      </>,
    );

    expect(screen.getByTestId("row").tagName).toBe("DIV");
    expect(screen.getByTestId("stack").tagName).toBe("DIV");
    expect(screen.getByTestId("card").tagName).toBe("DIV");
    expect(screen.getByTestId("row-li").tagName).toBe("LI");
    expect(screen.getByTestId("row-li")).toHaveClass("rf-row", "rf-gap-xs");
    expect(screen.getByTestId("stack-ul").tagName).toBe("UL");
    expect(screen.getByTestId("stack-ul")).toHaveClass("rf-stack", "rf-gap-md");
    expect(screen.getByTestId("card-li").tagName).toBe("LI");
    expect(screen.getByTestId("card-li")).toHaveClass("rf-card");
  });
});
