//  Copyright 2019 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

#include <float.h>

typedef struct histogram_S histogram_t;

extern histogram_t * histogram_new(uintptr_t, uintptr_t, uintptr_t);
extern uintptr_t * histogram_count(histogram_t *, uintptr_t);
extern void histogram_clear(histogram_t *);
extern void histogram_decr(histogram_t *, uintptr_t, uintptr_t);
extern void histogram_incr(histogram_t *, uintptr_t, uintptr_t);
extern uintptr_t histogram_percentile(histogram_t *, float);
extern uintptr_t histogram_samples(histogram_t *);
