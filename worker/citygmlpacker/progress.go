package citygmlpacker

import "sync"

const (
	progressResolution = 10000
)

type progress struct {
	steps int64

	mu      sync.RWMutex
	s, n, c int64
}

func (p *progress) Total() int64 {
	return p.steps * progressResolution
}

func (p *progress) Processed() int64 {
	p.mu.RLock()
	defer p.mu.RUnlock()
	if p.n == 0 {
		return p.s * progressResolution
	} else {
		return p.s*progressResolution + int64(float64(p.c)/float64(p.n)*progressResolution)
	}
}

func (p *progress) AddDep(deps int64) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.s += 1
	p.n = deps
	p.c = 0
}

func (p *progress) DepOne() {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.c += 1
}

func (p *progress) DepEnd() {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.s += 1
	p.n = 0
	p.c = 0
}

func (p *progress) Reset(le int64) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.s = 0
	p.n = 0
	p.c = 0
	p.steps = le
}
