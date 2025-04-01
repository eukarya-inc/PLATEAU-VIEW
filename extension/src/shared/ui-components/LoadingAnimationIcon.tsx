import { keyframes, styled } from "@mui/material";
import { FC } from "react";

const rotateAnimation = keyframes`
  100% { transform: rotate(1turn); }
`;

// Styled loader component
const Loader = styled("div")<{ size: number }>(({ size }) => ({
  width: size,
  height: size,
  aspectRatio: "1",
  display: "grid",
  borderRadius: "50%",
  background: `
    linear-gradient(0deg, rgba(0, 0, 0, 0.5) 30%, transparent 0 70%, rgba(0, 0, 0, 1) 0)
      50% / 12.5% 100%,
    linear-gradient(90deg, rgba(0, 0, 0, 0.25) 30%, transparent 0 70%, rgba(0, 0, 0, 0.75) 0)
      50% / 100% 12.5%
  `,
  backgroundRepeat: "no-repeat",
  animation: `${rotateAnimation} 1s infinite steps(8)`,
  opacity: 0.5,

  "&::before, &::after": {
    content: '""',
    gridArea: "1 / 1",
    borderRadius: "50%",
    background: "inherit",
  },

  "&::before": {
    opacity: 0.915,
    transform: "rotate(45deg)",
  },

  "&::after": {
    opacity: 0.83,
    transform: "rotate(90deg)",
  },
}));

export interface LoadingAnimationIconProps {
  size?: number;
}

export const LoadingAnimationIcon: FC<LoadingAnimationIconProps> = ({ size = 18 }) => (
  <Loader size={size} />
);
