import { alpha, IconButton, Slider, Stack, styled, Typography } from "@mui/material";
import { anchorRef, bindPopover, bindTrigger, usePopupState } from "material-ui-popup-state/hooks";
import { useCallback, useId, type FC, type MouseEvent } from "react";

import { AppToggleButton, type AppToggleButtonProps } from "./AppToggleButton";
import { FloatingPanel } from "./FloatingPanel";
import { DropDownIcon } from "./icons";
import { OverlayPopper } from "./OverlayPopper";

const StyledButton = styled(IconButton)(({ theme }) => ({
  marginRight: theme.spacing(-1.5),
  marginLeft: -2,
  padding: 0,
  color: alpha(theme.palette.text.primary, 0.5),
  "&&": {
    minWidth: 0,
  },
  "&:hover": {
    color: theme.palette.text.primary,
    backgroundColor: "transparent",
  },
}));

const ContentWrapper = styled("div")(({ theme }) => ({
  padding: theme.spacing(2),
  display: "flex",
  flexDirection: "column",
  gap: theme.spacing(1),
}));

export interface AppToggleButtonSliderItem {
  title: string;
  value: number;
  min: number;
  max: number;
  onValueChange?: (value: number) => void;
}

export interface AppToggleButtonSliderProps extends AppToggleButtonProps {
  item: AppToggleButtonSliderItem;
}

export const AppToggleButtonSlider: FC<AppToggleButtonSliderProps> = ({
  item,
  children,
  ...props
}) => {
  const id = useId();
  const popupState = usePopupState({
    variant: "popover",
    popupId: id,
  });
  const buttonProps = bindTrigger(popupState);
  const popoverProps = bindPopover(popupState);

  const { onClick } = buttonProps;
  const handleClick = useCallback(
    (event: MouseEvent) => {
      event.stopPropagation();
      onClick(event);
    },
    [onClick],
  );

  const handleSliderValueChange = useCallback(
    (_e: Event, newValue: number | number[]) => {
      const value = Array.isArray(newValue) ? newValue[0] : newValue;
      item.onValueChange?.(value);
    },
    [item],
  );

  return (
    <>
      <AppToggleButton
        {...props}
        component="div"
        title={!popoverProps.open ? props.title : undefined}
        ref={anchorRef(popupState)}>
        {children}
        <StyledButton {...bindTrigger(popupState)} onClick={handleClick}>
          <DropDownIcon />
        </StyledButton>
      </AppToggleButton>
      <OverlayPopper {...popoverProps} inset={1.5}>
        <FloatingPanel>
          <ContentWrapper>
            <Stack
              direction="row"
              spacing={1}
              alignItems="center"
              justifyContent="space-between"
              sx={{ width: 260 }}>
              <Typography variant="body1">{item.title}</Typography>
              <Typography variant="body1">{item.value}</Typography>
            </Stack>
            <Slider
              value={item.value}
              onChange={handleSliderValueChange}
              aria-label="Default"
              valueLabelDisplay="off"
              size="small"
              min={item.min}
              max={item.max}
              marks
            />
          </ContentWrapper>
        </FloatingPanel>
      </OverlayPopper>
    </>
  );
};
