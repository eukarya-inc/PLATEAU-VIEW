import { FC, ReactNode } from "react";

const textStyle = {
  padding: "5px",
  color: "#fff",
  backgroundColor: "#000",
  borderRadius: "5px",
  lineHeight: 1,
};

export const MeasurementText: FC<{ children: ReactNode }> = ({ children }) => {
  return <div style={textStyle}>{children}</div>;
};
