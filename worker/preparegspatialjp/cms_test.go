package preparegspatialjp

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestCityItem_FileName(t *testing.T) {
	m := &CityItem{
		CityNameEn: "hoge-shi",
		CityCode:   "99999",
		Year:       "2023年度",
		CodeLists:  "https://example.com/99999_hoge-shi_pref_2023_citygml_1_op_codelists.zip",
	}

	assert.Equal(t, "99999_hoge-shi_pref_2023_3dtiles_mvt_1_op", m.FileName("3dtiles_mvt", ""))
	assert.Equal(t, "99999_hoge-shi_pref_2023_3dtiles_mvt_1_op_hoge", m.FileName("3dtiles_mvt", "hoge"))
	assert.Equal(t, "99999_hoge-shi_pref_2023_3dtiles_mvt_1_op.zip", m.FileName("3dtiles_mvt", ".zip"))
}
