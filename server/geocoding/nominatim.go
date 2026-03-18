package geocoding

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"

	"github.com/eukarya-inc/jpareacode"
)

const DefaultNominatimURL = "https://nominatim.openstreetmap.org/reverse"

type NominatimClient struct {
	httpClient *http.Client
	url        string
	userAgent  string
}

type nominatimResponse struct {
	Address *nominatimAddress `json:"address"`
	Name    string            `json:"display_name"`
}

type nominatimAddress struct {
	City        string `json:"city"`
	Town        string `json:"town"`
	Village     string `json:"village"`
	County      string `json:"county"`
	State       string `json:"state"`
	ISO31662Lv4 string `json:"ISO3166-2-lvl4"`
	Country     string `json:"country"`
	CountryCode string `json:"country_code"`
}

func NewNominatimClient(httpClient *http.Client, url, userAgent string) *NominatimClient {
	if httpClient == nil {
		httpClient = http.DefaultClient
	}
	if url == "" {
		url = DefaultNominatimURL
	}
	if userAgent == "" {
		userAgent = "PLATEAU-VIEW"
	}
	return &NominatimClient{
		httpClient: httpClient,
		url:        url,
		userAgent:  userAgent,
	}
}

func (c *NominatimClient) Fetch(ctx context.Context, lon, lat float64) (*GSIResult, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	q := req.URL.Query()
	q.Set("format", "json")
	q.Set("lon", strconv.FormatFloat(lon, 'f', -1, 64))
	q.Set("lat", strconv.FormatFloat(lat, 'f', -1, 64))
	q.Set("zoom", "18")
	q.Set("addressdetails", "1")
	req.URL.RawQuery = q.Encode()

	req.Header.Set("User-Agent", c.userAgent)

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to send request: %w", err)
	}
	defer func() {
		_ = resp.Body.Close()
	}()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	var nomResp nominatimResponse
	if err := json.NewDecoder(resp.Body).Decode(&nomResp); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	if nomResp.Address == nil {
		return nil, nil
	}

	// Convert Nominatim response to GSIResult
	result, err := c.convertToGSIResult(&nomResp)
	if err != nil {
		return nil, fmt.Errorf("failed to convert response: %w", err)
	}

	return result, nil
}

func (c *NominatimClient) convertToGSIResult(resp *nominatimResponse) (*GSIResult, error) {
	addr := resp.Address

	// Only process Japan addresses
	if addr.CountryCode != "jp" {
		return nil, nil
	}

	// Get prefecture code from ISO3166-2-lvl4 (e.g., "JP-13" -> 13)
	prefCode := 0
	if addr.ISO31662Lv4 != "" {
		prefCodeStr := strings.TrimPrefix(addr.ISO31662Lv4, "JP-")
		if code, err := strconv.Atoi(prefCodeStr); err == nil {
			prefCode = code
		}
	}

	// Get city/ward name (Nominatim uses different fields depending on the area type)
	cityName := addr.City
	if cityName == "" {
		cityName = addr.Town
	}
	if cityName == "" {
		cityName = addr.Village
	}
	if cityName == "" {
		cityName = addr.County
	}

	if cityName == "" || prefCode == 0 {
		return nil, nil
	}

	// Look up municipality code using jpareacode
	city := jpareacode.CityByName(prefCode, cityName, "")
	if city == nil {
		// Try searching by name if exact match fails
		cities := jpareacode.CitiesByName(cityName)
		for _, c := range cities {
			if c.PrefCode == prefCode {
				city = &c
				break
			}
		}
	}

	if city == nil {
		return nil, nil
	}

	municipalityCode := jpareacode.FormatCityCode(city.Code())

	return &GSIResult{
		MunicipalityCode: municipalityCode,
		Name:             resp.Name,
	}, nil
}
