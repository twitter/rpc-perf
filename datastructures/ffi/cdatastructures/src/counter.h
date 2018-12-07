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

typedef struct counter_S counter_t;

extern counter_t * counter_new(void);
extern uintptr_t * counter_count(counter_t *);
extern void counter_clear(counter_t *);
extern void counter_decr(counter_t *, uintptr_t);
extern void counter_incr(counter_t *, uintptr_t);
