import { useEffect, useState } from "react";

import { cityGMLClient } from "..";
import { isNotNullish } from "../../../../prototypes/type-helpers";

export type UseCityGMLSpaceAttributesProps = {
  spaceZFXYStrs: string[];
  featureTypes: (string | null)[];
};

export default ({ spaceZFXYStrs, featureTypes }: UseCityGMLSpaceAttributesProps) => {
  const [attributes, setAttributes] = useState<Record<string, any>[] | null>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setAttributes(null);
    setLoading(false);
    setError(null);

    const types = featureTypes.filter(isNotNullish);

    if (!types || types.length === 0 || !spaceZFXYStrs || spaceZFXYStrs.length === 0) {
      return;
    }

    const fetchAttributes = async () => {
      try {
        setLoading(true);
        setError(null);
        setAttributes(null);

        const data = await cityGMLClient?.getAttributesBySpace({
          spaceZFXYStrs,
          types,
        });
        setAttributes(data ?? null);
      } catch (err: any) {
        setError(err.message || "An error occurred while fetching attributes.");
      } finally {
        setLoading(false);
      }
    };

    fetchAttributes();
  }, [spaceZFXYStrs, featureTypes]);

  return {
    attributes,
    loading,
    error,
  };
};
