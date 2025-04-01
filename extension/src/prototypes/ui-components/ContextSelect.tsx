import AddIcon from "@mui/icons-material/AddOutlined";
import {
  filledInputClasses,
  Select,
  selectClasses,
  Stack,
  styled,
  Typography,
  menuClasses,
  type SelectChangeEvent,
  type SelectProps,
} from "@mui/material";
import {
  forwardRef,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { mergeRefs } from "react-merge-refs";
import invariant from "tiny-invariant";

const StyledSelect = styled(Select)(({ theme }) => ({
  flexGrow: 0,
  flexShrink: 0,
  height: theme.spacing(5),
  [`& .${selectClasses.select}`]: {
    // Increase specificity
    [`&.${selectClasses.select}`]: {
      height: "100%",
      [`&.${filledInputClasses.input}`]: {
        paddingRight: theme.spacing(3.5),
      },
    },
  },
  [`& .${selectClasses.icon}`]: {
    right: 4,
  },
})) as unknown as typeof Select; // For generics
const Badge = styled("div")(({ theme }) => ({
  ...theme.typography.caption,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  width: 18,
  height: 18,
  marginRight: -1,
  marginLeft: -1,
  borderRadius: "999px",
  color: theme.palette.getContrastText(theme.palette.primary.dark),
  backgroundColor: theme.palette.primary.light,
  fontWeight: 700,
  lineHeight: 1,
}));
const StyledAddIcon = styled(AddIcon)({
  fontSize: 16,
});
export interface ContextSelectProps extends SelectProps<string[]> {
  label: ReactNode;
  autoClose?: boolean;
}
export const ContextSelect = forwardRef<HTMLButtonElement, ContextSelectProps>(
  ({ label, autoClose = true, children, onChange, ...props }, forwardedRef) => {
    const [open, setOpen] = useState(false);
    const [measured, setMeasured] = useState(false);
    const handleChange = useCallback(
      (event: SelectChangeEvent<string[]>, value: ReactNode) => {
        invariant(Array.isArray(event.target.value));
        onChange?.(event, value);
        if (autoClose) {
          setOpen(false);
        }
      },
      [autoClose, onChange],
    );
    const renderValue = useCallback(
      (value: string[]) =>
        value.length === 0 ? (
          <Stack direction="row" spacing={0.75} alignItems="center">
            <StyledAddIcon color="action" />
            <Typography variant="body2">{label}</Typography>
          </Stack>
        ) : (
          <Stack direction="row" spacing={0.75} alignItems="center">
            <Badge>{value.length}</Badge>
            <Typography variant="body2" color="primary">
              {label}
            </Typography>
          </Stack>
        ),
      [label]
    );
    const ref = useRef<HTMLButtonElement>(null);
    const menuRef = useRef<HTMLUListElement>(null);
    const [menuStyle, setMenuStyle] = useState<{ top: number; left: number }>({
      top: 0,
      left: 0,
    });
    useEffect(() => {
      const handler = (event: MouseEvent): void => {
        if (!(event.target instanceof Node)) {
          return;
        }
        if (ref.current?.contains(event.target) === true) {
          setOpen(true);
          setMeasured(false);
        } else if (
          menuRef.current?.contains(event.target) !== true &&
          menuRef.current?.parentElement !== event.target
        ) {
          setOpen(false);
        }
      };
      window.addEventListener("mouseup", handler);
      return () => {
        window.removeEventListener("mouseup", handler);
      };
    }, []);
    const menuProps = useMemo(
      () => ({
        ...props.MenuProps,
        anchorReference: "anchorPosition" as const,
        anchorPosition: menuStyle,
        transformOrigin: {
          vertical: "top" as const,
          horizontal: "left" as const,
        },
        sx: {
          ...props.MenuProps?.sx,
          [`& .${menuClasses.paper}`]: {
            maxWidth: 700,
            position: "fixed",
            opacity: measured ? 1 : 0,
            transition: "opacity 0.2s ease-in-out",
          },
        },
        MenuListProps: {
          ...props.MenuProps?.MenuListProps,
          ref: menuRef,
        },
        TransitionProps: {
          onEnter: (node: HTMLElement) => {
            if (ref.current && node) {
              const rect = ref.current.getBoundingClientRect();
              const width = node.getBoundingClientRect().width;
              setMenuStyle({
                top: rect.bottom,
                left: (rect.right - rect.left) / 2 + rect.left - width / 2,
              });
              setMeasured(true);
            }
          },
        },
      }),
      [menuStyle, props.MenuProps, measured]
    );
    return (
      <StyledSelect
        ref={mergeRefs([ref, forwardedRef])}
        open={open}
        variant="filled"
        size="small"
        multiple
        displayEmpty
        renderValue={renderValue}
        {...props}
        onChange={handleChange}
        MenuProps={menuProps}
      >
        {children}
      </StyledSelect>
    );
  }
);
